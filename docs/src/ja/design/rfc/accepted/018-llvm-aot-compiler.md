---
title: 'RFC-018：LLVM AOT コンパイラ設計'
status: '受入済み'
author: '晨煦'
created: '2026-02-15'
updated: '2026-07-05（GitHub Issue #14、#134 と同期；実装状況分析を追加）'
issue: '#14'
tracking_issue: 'https://github.com/ChenXu233/YaoXiang/issues/134'
---

# RFC-018：LLVM AOT コンパイラ設計

> **参照**:
>
> - [RFC-024：spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 並行モデルとスケジューラの分離設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
> - [RFC-026：FFI コア機構](./026-ffi-core-mechanism.md)
> - [RFC-010：統一型構文](../accepted/010-unified-type-syntax.md)

> **廃止**:
>
> - 旧版「ボトムアップ自動 DAG 分析」モデル — RFC-024 spawn ブロック直接部分式モデルに置き換え
> - `@IO`/`@Pure` 暗黙的副作用推論 — RFC-024 リソース型機構に置き換え
> - `Arc(T)` 型マッピング — RFC-009 v9 `ref` キーワードに置き換え

## 概要

本文書は YaoXiang 言語の LLVM
AOT（Ahead-of-Time）コンパイラを設計する。LLVM バックエンドと VM バックエンド（インタプリタ）は同一のコンパイルフロントエンドを共有し、[RFC-008](../accepted/008-runtime-concurrency-model.md)
で定義された双方向バックエンドアーキテクチャを構成する：VM は開発デバッグ用、LLVM は本番リリース用。

**コア責務**：

```
ソースコード → フロントエンド（共有）→ IR → LLVM Codegen → .o → スケジューラ静的ライブラリをリンク → exe
```

コンパイラは YaoXiang ソースコードをネイティブ機械語にコンパイルする：

| 言語機能                 | コンパイル戦略                                                                                    |
| ------------------------ | ------------------------------------------------------------------------------------------------- |
| 通常コード               | 順序機械語、スケジューリングオーバーヘッドゼロ                                                    |
| `spawn { }` ブロック     | 直接部分式 → タスク分发 + 同期待機（[RFC-024](../accepted/024-concurrency-model.md) に整合）      |
| `native("symbol")`       | LLVM `declare external` + パラメータ marshalling（[RFC-026](./026-ffi-core-mechanism.md) に整合） |
| `.drop` 破棄             | RAII cleanup コード挿入（[RFC-009](../accepted/009-ownership-model.md) に整合）                   |
| `&T` / `&mut T` トークン | ゼロサイズ型、コンパイル後に消失                                                                  |
| `ref T` 共有             | `{ refcount_ptr, data_ptr }` ファットポインタ、コンパイラが Rc/Arc を自動選択                     |

**RFC-024 との関係**：RFC-024 は spawn ブロックの**ユーザ意味論**を定義した（直接部分式でタスク作成、同期ブロッキング待機）。本文書はこれらの意味論を**機械語にコンパイルする方法**を定義する。

**RFC-026 との関係**：RFC-026 は FFI の**ユーザ構文**を定義した（`native()`、`[0]`
メソッドバインディング、`.drop`）。本文書は FFI 呼び出しが**LLVM IR を生成する方法**を定義する。

---

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在の YaoXiang は実行バックエンドとしてインタプリタのみを持つ：

| 問題             | 影響                                       |
| ---------------- | ------------------------------------------ |
| 性能ボトルネック | 解釈実行は機械語より 10-100 倍遅い         |
| デプロイが複雑   | インタプリタとランタイムの携带が必要       |
| 本番環境         | インタプリタは性能敏感的シナリオに適さない |

### 双方向バックエンドモデルにおける LLVM

[RFC-008](../accepted/008-runtime-concurrency-model.md)
§6 は双方向バックエンドアーキテクチャを定義した：

```
                    ┌─────────────────────┐
                    │   コンパイルフロントエンド（統一）     │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn 分析       │
                    │   → エスケープ分析          │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM バックエンド（開発）   │     │  LLVM バックエンド（本番）  │
      │   IR → 解釈実行    │     │  IR → ネイティブコード      │
      │   ステップデバッグ         │     │  スケジューラ静的ライブラリをリンク   │
      │   高速イテレーション         │     │  .exe 出力         │
      └───────────────────┘     └───────────────────┘
```

2 つのバックエンドの**動作は完全に一致**する——違いは実行方法のみ。同じソースコード、同じ型検査、同じ spawn 分析結果。

---

## 提案

### 1. コンパイラアーキテクチャ

LLVM バックエンドはコンパイルパイプラインの最終段階に位置し、フロントエンドから IR を受け取り、ネイティブコードを生成する：

```
ソースコード
  → Lexer / Parser（frontend/core/）
  → TypeCheck + spawn 分析（frontend/core/typecheck/）
  → IR 生成（middle/core/ir_gen.rs）
  → LLVM Codegen（backends/llvm/）
      ├── 型マッピング：YaoXiang 型 → LLVM IR 型
      ├── 関数翻訳：IR 命令 → LLVM IR 命令
      ├── spawn 展開：直接部分式 → タスク関数 + スケジューリング呼び出し
      ├── FFI 展開：native() 呼び出し → declare + marshalling
      └── 破棄挿入：スコープ終了 → .drop() 呼び出し
  → LLVM 最適化 + ターゲットコード生成
  → ランタイム静的ライブラリをリンク → 実行可能ファイル
```

### 2. コンパイルフロー

```
Phase 1: フロントエンド（VM バックエンドと共有）
  - 解析、型検査、spawn ブロック分析、エスケープ分析
  - 出力：型注釈付き IR

Phase 2: LLVM IR 生成
  - 型マッピング、関数宣言、命令翻訳
  - 出力：LLVM Module

Phase 3: LLVM 最適化
  - 標準 LLVM 最適化パイプライン（O0/O1/O2/O3）
  - インライン、定数畳み込み、デッドコード除去

Phase 4: ターゲットコード生成
  - LLVM TargetMachine → .o ファイル
  - プラットフォーム：Linux (ELF)、macOS (Mach-O)、Windows (COFF)

Phase 5: リンク
  - ランタイム静的ライブラリをリンク（スケジューラ、アロケータ）
  - 出力：実行可能ファイル
```

### 3. 型マッピング

#### 3.1 YaoXiang → LLVM IR 型マッピング

| YaoXiang 型  | LLVM IR 型                    | 説明                                                      |
| ------------ | ----------------------------- | --------------------------------------------------------- |
| `Int`        | `i64`                         | デフォルト 64 ビット符号付き整数                          |
| `Int32`      | `i32`                         | 明示的 32 ビット整数（主に FFI 用）                       |
| `Float`      | `f64`                         | デフォルト 64 ビット浮動小数点                            |
| `Float32`    | `f32`                         | 明示的 32 ビット浮動小数点（主に FFI 用）                 |
| `Bool`       | `i1`                          | ブール値                                                  |
| `Char`       | `i32`                         | Unicode コードポイント                                    |
| `String`     | `{ i8*, i64 }`                | ポインタ + バイト長                                       |
| `Void`       | `{}`                          | ゼロサイズ空型                                            |
| `&T`         | —                             | ゼロサイズトークン、コンパイル後に消失、IR を生成しない   |
| `&mut T`     | —                             | ゼロサイズトークン、コンパイル後に消失、IR を生成しない   |
| `ref T`      | `{ i64*, T* }`                | ファットポインタ（参照カウントポインタ + データポインタ） |
| `*T`         | `T*`                          | 生ポインタ                                                |
| `[T; N]`     | `[N x T]`                     | 固定長配列                                                |
| `List(T)`    | `{ T*, i64, i64 }`            | データポインタ + 長さ + 容量                              |
| 構造体       | 対応 LLVM struct              | フィールドは定義順でレイアウト                            |
| 記録列挙     | `{ i64, [max_payload_size] }` | タグ + 最大 payload の共用体                              |
| `?T`         | `{ i1, T }`                   | 有値タグ + データ（汎用表現）                             |
| FFI 不透明型 | `{ i8* }`                     | C ポインタをラップ                                        |
| 関数ポインタ | `T (...)*`                    | 関数ポインタ型                                            |

> **`&T` / `&mut T` ゼロランタイムオーバーヘッド**：[RFC-009](../accepted/009-ownership-model.md)
> §2.7 はコンパイラ内部でトークンにブランド識別子（コンパイル時一意整数）を割り当て、モノ]~!b[モルフィゼーションとインライン後にブランドが完全に消失することを定義——生成された機械語にはトークンの痕跡は存在しない。

#### 3.2 FFI パラメータ型マッピング

[RFC-026](./026-ffi-core-mechanism.md) §2.2 に整合、LLVM IR 列を追加：

| C 型                 | YaoXiang 型     | LLVM IR        | 説明                                    |
| -------------------- | --------------- | -------------- | --------------------------------------- |
| `int`                | `Int32`         | `i32`          |                                         |
| `long`               | `Int64`         | `i64`          |                                         |
| `float`              | `Float32`       | `f32`          |                                         |
| `double`             | `Float64`       | `f64`          |                                         |
| `char`               | `Char`          | `i32`          | C char → YaoXiang Char（Unicode 互換）  |
| `char*`              | `String`        | `{ i8*, i64 }` | marshalling：C string → YaoXiang String |
| `bool`               | `Bool`          | `i1`           |                                         |
| `size_t`             | `Uint`          | `i64`          |                                         |
| `void*`              | `*Void`         | `i8*`          |                                         |
| `struct T*`          | `T`（透過型）   | `T*`           | ポインタで 전달                         |
| `typedef struct T T` | `T`（不透明型） | `{ i8* }`      | C ポインタをラップ                      |

### 4. IR 正規化と命令翻訳

#### 4.0 IR 正規化（スタック → レジスタ）

現在の IR（`src/middle/core/ir.rs`）にはスタック操作命令（`Push`/`Pop`/`Dup`/`Swap`）が含まれる。これはバイトコード VM 用に設計されている。LLVM
IR は SSA 形式であり、スタック操作を受け入れない。

**処理戦略**：LLVM パスでは命令翻訳の前に、轻量な正規化パスを経由する：

| スタック命令 | 正規化戦略                                               |
| ------------ | -------------------------------------------------------- |
| `Push(r)`    | `stack.push(r)` を記録、IR を生成しない                  |
| `Pop(r)`     | `r = stack.pop()`、`load` を生成（スタックスロットから） |
| `Dup`        | `stack.push(stack.top())`、IR を生成しない               |
| `Swap`       | スタックトップの 2 要素を交換、IR を生成しない           |

正規化後、すべてのオペランドがレジスタ/ローカル変数参照になり、スタック操作が完全に消除される。このパスは
`translator.rs` の第一步として実行する。

> **なぜ IR レベルでスタック命令を消除しないのか？**
> VM バックエンドはスタック意味論を必要とするから。LLVM 翻訳入口で正規化することで、IR が 2 つのバックエンド間で共有されるようにする——各バックエンドは同一の IR を自分の必要に応じて消費する。
>
> **前提**：IR 生成段階ではスタックバランスを保証する——すべての制御フロー経路が同一プログラム点に到达する時のスタック深さは一致する（VM バイトコードバックエンドは同じ前提に依存，否则字节码执行会出错）。正規化パスはこの前提をチェックしない；违反した場合は LLVM バックエンドは未定義動作を生成する。

#### 4.1 命令翻訳表

以下は `Instruction` 列挙体の各バリアントの LLVM IR 翻訳戦略を逐一列出する。命令名は
`src/middle/core/ir.rs` と完全に一致する。

**算術命令**：

| IR 命令                 | LLVM IR                                    | 説明                                     |
| ----------------------- | ------------------------------------------ | ---------------------------------------- |
| `Add { dst, lhs, rhs }` | `add`（整数）/ `fadd`（浮動小数点）        | 型に応じて整数または浮動小数点加算を選択 |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub`                             |                                          |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul`                             |                                          |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv`                   | 符号付き/符号なし/浮動小数点除法         |
| `Mod { dst, lhs, rhs }` | `srem` / `urem`                            | 符号付き/符号なし剰余                    |
| `Neg { dst, src }`      | `sub 0, src`（整数）/ `fneg`（浮動小数点） |                                          |

**ビット演算命令**：

| IR 命令                 | LLVM IR | 説明         |
| ----------------------- | ------- | ------------ |
| `And { dst, lhs, rhs }` | `and`   |              |
| `Or { dst, lhs, rhs }`  | `or`    |              |
| `Xor { dst, lhs, rhs }` | `xor`   |              |
| `Shl { dst, lhs, rhs }` | `shl`   | 左シフト     |
| `Shr { dst, lhs, rhs }` | `lshr`  | 論理右シフト |
| `Sar { dst, lhs, rhs }` | `ashr`  | 算術右シフト |

**比較命令**：

| IR 命令                | LLVM IR                 | 説明 |
| ---------------------- | ----------------------- | ---- |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq`  |      |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one`  |      |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` |      |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` |      |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` |      |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` |      |

**制御フロー命令**：

| IR 命令                 | LLVM IR                                     | 説明       |
| ----------------------- | ------------------------------------------- | ---------- |
| `Jmp(label)`            | `br label %L`                               | 無条件分岐 |
| `JmpIf(cond, label)`    | `br i1 %cond, label %L, label %fallthrough` | 条件分岐   |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | 条件不分岐 |
| `Ret(Some(v))`          | `ret T %v`                                  | 戻り値あり |
| `Ret(None)`             | `ret void`                                  | 戻り値なし |

**呼び出し命令**：

| IR 命令                                    | LLVM IR                             | 説明                                    |
| ------------------------------------------ | ----------------------------------- | --------------------------------------- |
| `Call { dst, func, args }`                 | `%r = call T @func(...)`            | 静的呼び出し                            |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call`（関数ポインタ） | 仮想メソッド呼び出し、vtable で検索     |
| `CallDyn { dst, func, args }`              | `%r = call T %func(...)`            | 動的呼び出し（クロージャ/関数ポインタ） |
| `TailCall { func, args }`                  | `musttail call` / `tail call`       | 末尾呼び出し最適化                      |

**メモリ命令**：

| IR 命令                               | LLVM IR                                                           | 説明                                                   |
| ------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------ |
| `Move { dst, src }`                   | —                                                                 | 正規化後レジスタコピーに变化、SSA 構築で大部分消除可能 |
| `Load { dst, src }`                   | `%v = load T, T* %src`                                            |                                                        |
| `Store { dst, src }`                  | `store T %src, T* %dst`                                           |                                                        |
| `Alloc { dst, size }`                 | `%p = alloca T`（スタック）/ `call @malloc`（ヒープにエスケープ） | エスケープ分析が配置位置を決定                         |
| `Free(ptr)`                           | `call @free(%ptr)`（ヒープ）/ —（スタック、自動回収）             |                                                        |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]`（スタック）/ `call @malloc`（ヒープ）       |                                                        |

**構造体/配列アクセス命令**：

| IR 命令                                   | LLVM IR                                                | 説明                              |
| ----------------------------------------- | ------------------------------------------------------ | --------------------------------- |
| `LoadField { dst, src, field }`           | `%ptr = getelementptr T, T* %src, 0, field` + `load`   |                                   |
| `StoreField { dst, field, src }`          | `%ptr = getelementptr T, T* %dst, 0, field` + `store`  |                                   |
| `LoadIndex { dst, src, index }`           | `%ptr = getelementptr T, T* %src, 0, %index` + `load`  |                                   |
| `StoreIndex { dst, index, src }`          | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` |                                   |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` チェーン                                 | フィールド順で LLVM struct を構築 |

**型変換命令**：

| IR 命令                          | LLVM IR                                                                                                     | 説明                                                          |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- |
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | ソース/ターゲット型の組み合わせに応じて適切な cast 命令を選択 |
| `TypeTest(val, type)`            | —                                                                                                           | コンパイル時型テスト、型タグを比較する `icmp eq` を生成       |

**所有権と借用命令**：

| IR 命令                        | LLVM IR                                              | 説明                                                              |
| ------------------------------ | ---------------------------------------------------- | ----------------------------------------------------------------- |
| `Borrow { dst, src, mutable }` | —                                                    | **ゼロサイズトークン、コンパイル後に完全に消失**、IR を生成しない |
| `Release(val)`                 | —                                                    | **ゼロサイズトークン、コンパイル後に完全に消失**                  |
| `Move { dst, src }`            | —                                                    | 所有権移動、正規化後レジスタコピーに变化                          |
| `Drop(val)`                    | `call void @T.drop(T* %val)`                         | 型のデストラクタ関数を呼び出す（§7 参照）                         |
| `ShareRef { dst, src }`        | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | コンパイラがスレッド間を自動選択 Arc/Rc                           |
| `ArcNew { dst, src }`          | `call %T* @Arc_new(%src)`                            | アトミック参照カウント = 1                                        |
| `ArcClone { dst, src }`        | `call %T* @Arc_clone(%src)`                          | アトミック参照カウント 증가                                       |
| `ArcDrop(val)`                 | `call void @Arc_drop(%val)`                          | アトミックデクリメント + 条件解放                                 |

**並行命令**：

| IR 命令                            | LLVM IR                            | 説明                                                      |
| ---------------------------------- | ---------------------------------- | --------------------------------------------------------- |
| `Spawn { closures, plan, result }` | スケジューラ呼び出しシリーズに展開 | 詳細 §5、ランタイム `task_spawn` + `task_wait_all`        |
| `Yield`                            | —                                  | AOT パスでは spawn ブロックは同期待機、yield は不要；無視 |

**unsafe ブロックと生ポインタ命令**：

| IR 命令                   | LLVM IR                                                      | 説明                                    |
| ------------------------- | ------------------------------------------------------------ | --------------------------------------- |
| `UnsafeBlockStart`        | —                                                            | **コンパイル時マーク、IR を生成しない** |
| `UnsafeBlockEnd`          | —                                                            | **コンパイル時マーク、IR を生成しない** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64`（またはポインタを直接コピー） |                                         |
| `PtrDeref { dst, src }`   | `%v = load T, T* %src`                                       |                                         |
| `PtrStore { dst, src }`   | `store T %src, T* %dst`                                      |                                         |
| `PtrLoad { dst, src }`    | `%v = load T, T* %src`                                       |                                         |

**文字列命令**：

| IR 命令                             | LLVM IR                                     | 説明                                         |
| ----------------------------------- | ------------------------------------------- | -------------------------------------------- |
| `StringLength { dst, src }`         | `%len = extractvalue { i8*, i64 } %src, 1`  | String は `{ ptr, len }`、長さはフィールド 1 |
| `StringConcat { dst, lhs, rhs }`    | `call String @yx_string_concat(%lhs, %rhs)` | ランタイムヘルパ関数                         |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32`                | 境界値チェックを含む                         |
| `StringFromInt { dst, src }`        | `call String @yx_string_from_int(%src)`     | ランタイムヘルパ関数                         |
| `StringFromFloat { dst, src }`      | `call String @yx_string_from_f64(%src)`     | ランタイムヘルパ関数                         |

**クロージャ命令**：

| IR 命令                                  | LLVM IR                                                               | 説明                                  |
| ---------------------------------------- | --------------------------------------------------------------------- | ------------------------------------- |
| `MakeClosure { dst, func: String, env }` | クロージャ構造体を割り当て + 関数ポインタ（関数名で検索）と環境を填充 | `{ fn_ptr, env_fields... }`           |
| `LoadUpvalue { dst, upvalue_idx }`       | `%v = extractvalue %env, upvalue_idx`                                 | クロージャ環境から upvalue を読み込む |
| `StoreUpvalue { src, upvalue_idx }`      | `%env = insertvalue %env, %src, upvalue_idx`                          | クロージャ環境に書き込む              |
| `CloseUpvalue(val)`                      | スタック上の upvalue をヒープにコピー                                 |                                       |

**その他の命令**：

| IR 命令                         | LLVM IR                                       | 説明                    |
| ------------------------------- | --------------------------------------------- | ----------------------- |
| `HeapAlloc { dst, type_id }`    | `call i8* @malloc(i64 size)` + 型タグ書き込み | ヒープ割り当て + 型情報 |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)`      | ランタイムヘルパ関数    |

> **注意**：`Push`/`Pop`/`Dup`/`Swap`
> は §4.0 正規化段階で消除已经ため、翻訳表には出现しない。`Borrow`/`Release`
> はゼロサイズのコンパイル時トークンであり、任何な機械語を生成しない。

### 5. spawn ブロックコード生成

[RFC-024](../accepted/024-concurrency-model.md)
に整合、spawn ブロックのコンパイルは以下のステップに分ける。

#### 5.1 意味論の振り返り

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // 直接部分式 → タスク 1
    t2 = fetch("url2"),   // 直接部分式 → タスク 2
    return (t1, t2)       // 同期待機、結果を組立
}
```

**ルール**（RFC-024 §2.1）：

- spawn ブロックの**直接部分式**（トップレベルのカンマ区切りステートメント）が并行タスクを生成する
- ネストされた `{}` 内の式は直接部分式ではなく、独立したタスクにならない
- spawn ブロック全体が同期ブロッキングし、すべてのタスク完了後に返回する

#### 5.2 コンパイルステップ

```
Step 1: 直接部分式の識別
  spawn ブロック本体を遍歴し、トップレベルステートメントを収集

Step 2: 依存関係分析
  各直接部分式について，前面のタスクが生成した変数をどの程度参照するかを分析
  依存なし → 即座に并行スケジューリング可能
  依存あり → 依存タスクの完了を待機

Step 3: リソース競合検出（RFC-024 §2.5）
  同一リソース型のインスタンスが複数のタスクに使用されているかを確認
  同一インスタンス競合 → 直列実行順序をマーク

Step 4: タスク関数の生成
  各直接部分式から独立した LLVM 関数（クロージャ）を生成

Step 5: スケジューリングコードの生成
  ランタイムスケジューラの task_spawn / task_wait を呼び出す

Step 6: 結果の組立
  すべてのタスク出力を収集し、return タプルを組立
```

#### 5.3 LLVM IR 生成パターン

```llvm
; spawn ブロック入口
%task_count = 2
%tasks = alloca [2 x %TaskHandle]

; タスク 1 を生成：fetch("url1")
%task1_fn = @spawn_closure_1
call @runtime_task_spawn(%tasks[0], %task1_fn, ...)

; タスク 2 を生成：fetch("url2")
%task2_fn = @spawn_closure_2
call @runtime_task_spawn(%tasks[1], %task2_fn, ...)

; すべてのタスクを同期待機
call @runtime_task_wait_all(%tasks, %task_count)

; 戻り値を組立
%r1 = call @runtime_task_result(%tasks[0])
%r2 = call @runtime_task_result(%tasks[1])
ret { %r1, %r2 }
```

#### 5.4 依存タスク

```yaoxiang
result = spawn {
    data = fetch("url"),       // タスク 1：依存なし
    processed = parse(data),   // タスク 2：タスク 1 の data に依存
    return processed
}
```

コンパイラは `parse(data)` がタスク 1 が生成した `data`
を参照していることを検出し、スケジューリングコード生成時に依存関係をマークする：

```llvm
; タスク 2 はタスク 1 への依存を持って生成
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 依存タスク 0（fetch）完了
```

#### 5.5 リソース型の自動直列化

[RFC-024 §2.5](../accepted/024-concurrency-model.md)
で定義されたリソース型（`FilePath`、`HttpUrl`、`DBUrl`、`Console`
およびユーザ定義リソース型）は spawn ブロック内で自動的に直列化される：

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // SqliteDb を使用（リソース型）
    r2 = db.exec("INSERT ...")    // 同一インスタンス → 自動直列化
}
```

コンパイラは同一リソースインスタンスが 2 つのタスクに使用されていることを検出し、直列化依存関係を生成する：

```llvm
; タスク 2 はタスク 1 に依存（同一リソース自動直列化）
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for データ並列

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

コンパイラは N 個の独立タスク（N = items の長さ）に展開し、最大并发数に制約される。

### 6. FFI コード生成

> ⚠️ **依存関係の説明**：本節で定義される FFI コード生成**アーキテクチャ**（`native("x")` →
> `declare external @x` → marshalling ラッパー関数 →
> call）は安定しており、RFC-026 の構文変更不影响。特定のパラメータ marshalling ルール表（§6.2）と不透明型レイアウト（§6.3）は RFC-026 の定義を参照——RFC-026 の
> `native()`
> 構文または marshalling ルールが変更された場合でも、本文書の対応するマッピング表を更新するだけでよく、アーキテクチャ層は影響を受けない。RFC-026 の現在状態：**審査中**、本文書と同じ
> `review/` ディレクトリにある。
>
> **受入前提条件**：本 RFC を受入する前に、RFC-026 の本文書 §6 に関連する部分（`native()`
> 宣言構文、パラメータ marshalling ルール、不透明型 `{ i8* }` レイアウト、`.drop`
> バインディング規約）は先に凍結するか、026 と一緒に受入すること。否则 §6.2/§6.3/§7 のマッピング表が実装前に時代遅れになる可能性がある。

[RFC-026](./026-ffi-core-mechanism.md) に整合、本節では FFI 呼び出しの LLVM IR 生成戦略を定義する。

#### 6.1 native() 関数宣言

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

LLVM IR にコンパイル：

```llvm
; 外部 C 関数を宣言
declare i8* @sqlite3_open(i8*)

; YaoXiang ラッパー関数（marshalling を処理）
define { i8* } @__yx_sqlite3_open({ i8*, i64 } %filename) {
    ; marshalling: YaoXiang String → C string
    %c_str = extractvalue { i8*, i64 } %filename, 0
    ; C 関数を呼び出す
    %raw = call i8* @sqlite3_open(i8* %c_str)
    ; unmarshalling: C ポインタ → 不透明型
    %result = insertvalue { i8* } undef, i8* %raw, 0
    ret { i8* } %result
}
```

**重要なポイント**：

- `native("sqlite3_open")` → `declare external @sqlite3_open`
- コンパイラが marshalling ラッパー関数を自動生成
- ラッパー関数のシグネチャは YaoXiang 型を使用し、内部で C 型に変換

#### 6.2 パラメータ Marshalling

| 方向                                     | 変換                                  |
| ---------------------------------------- | ------------------------------------- |
| YaoXiang `String` → C `char*`            | `.ptr` フィールドを抽出して 전달      |
| YaoXiang `Int32` → C `int`               | 直接 전달（`i32`）                    |
| YaoXiang `*Void` → C `void*`             | 直接 전달（`i8*`）                    |
| YaoXiang `T`（透過型） → C `struct T*`   | アドレスを取って 전달                 |
| YaoXiang `T`（不透明型） → C `struct T*` | `{ i8* }` からポインタを抽出して 전달 |

#### 6.3 不透明型の LLVM レイアウト

[RFC-026](./026-ffi-core-mechanism.md) §4.1 で定義された不透明型：

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVM レイアウト：`{ i8* }` — C ポインタを包含する構造体。

**レイアウト最適化**：不透明型が単一の `handle: *Void` フィールドのみを持つ場合、`i8*`
を直接使用するように最適化できる（外側 struct を省略）。最適化後の ABI は C ポインタと完全に一致し、ゼロ marshalling オーバーヘッド。コンパイラはデフォルトでこの最適化を有効にし、ユーザーは認識しない。

#### 6.4 ?T nullable 戻り値の LLVM 表現

[RFC-026](./026-ffi-core-mechanism.md) §7.6 で定義された FFI nullable 戻り値：

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

汎用 LLVM 表現：`{ i1, { i8* } }` — 有値タグ + データ。

**FFI null ポインタ向け最適化**：`?T` の `T` が不透明型（内部がポインタ）の場合、コンパイラは
**null ポインタ = None** 最適化を使用：

```llvm
; 最適化後の LLVM 表現：null 可能なポインタを直接使用
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None、非 null → Some（不透明型にラップ）
    ret i8* %raw
}
```

呼び出し側：

```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

この最適化により `?SqliteDb`
の FFI 呼び出しは**追加オーバーヘッドゼロ**——C の null チェックと完全に同値。

#### 6.5 yx-bindgen 統合

[yx-bindgen](./026-ffi-core-mechanism.md)
§6 で自動生成されたバインディングファイルはコンパイル時に通常の YaoXiang ソースコードとして処理される。コンパイラはコードが bindgen から来ていることを知る必要がない——`native()`
宣言と `unsafe {}` 型定義の処理方法は完全に同じ。

### 7. デストラクタコード生成

[RFC-009](../accepted/009-ownership-model.md) の RAII 意味論と
[RFC-026](./026-ffi-core-mechanism.md) §7 の `.drop` 規約に整合。

#### 7.1 .drop バインディング識別

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

コンパイラは `.drop` バインディングを識別し、型メタデータにデストラクタ関数ポインタをマークする。

#### 7.2 スコープ終了時の Cleanup 挿入

```
ユーザーコード：
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← スコープ終了
}

コンパイラが挿入した cleanup（逆順）：
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**挿入位置**：

- 通常のスコープ終了（`}`）
- 早期リターン（`return` 前）
- `?` エラー伝播パス（`?` 前）
- spawn ブロック終了（タスク内変数の破棄）

#### 7.3 Move と破棄

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有権を db2 に移動
// db は無効化済み，这里不为 db 插入 drop
// ← スコープ終了：db2 にのみ drop を挿入
```

コンパイラは Move 意味論（[RFC-009](../accepted/009-ownership-model.md)
§1）を追踪し、変数の最終保持者にのみデストラクタ呼び出しを挿入する。

#### 7.4 破棄失敗処理

```llvm
; debug モード：デストラクタ戻り値をチェック
%ret = call i32 @sqlite3_close(i8* %handle)
%ok = icmp eq i32 %ret, 0
br i1 %ok, label %done, label %panic
panic:
  call @__yx_panic("destructor failed")
  unreachable
done:
  ret void

; release モード：戻り値を無視
call i32 @sqlite3_close(i8* %handle)
ret void
```

### 8. コンパイル生成物の構造

コンパイル生成物には以下のコンポーネントが含まれる（具体的な struct 定義は実装段階で確定）：

- **機械語**：LLVM がコンパイルしたターゲットファイル（`.o`）、すべての関数翻訳結果を含む
- **spawn メタデータ**：各 spawn ブロックのタスク関数ポインタ、依存関係、リソース競合直列化ペア
- **FFI シンボルテーブル**：外部 C シンボル参照（シンボル名 + 弱参照かどうか）
- **エントリポイントテーブル**：実行可能ファイルのエントリ関数リスト
- **型情報**：リフレクションメタデータ、`.reflect` セグメントに書き込み、ランタイムで必要時に mmap

### 9. ランタイムライブラリ

[RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md)
に整合、ランタイムは**静的ライブラリ**として最終 exe にリンクされる。

```
最終 exe の内部構造：

┌────────────────────────────────────────────┐
│  ユーザーコード（ネイティブ機械語）                       │
│  ├── 通常関数（順序実行）                    │
│  ├── spawn ブロック展開（タスク関数 + スケジューリング呼び出し）     │
│  ├── FFI marshalling ラッパー関数               │
│  └── RAII 破棄コード                          │
├────────────────────────────────────────────┤
│  ランタイム静的ライブラリ（约 500KB-1MB、プラットフォームと機能選択に依存）  │
│  ├── スレッドプール（num_workers）                  │
│  ├── イベントループ（libuv / io_uring）           │
│  ├── ワークスティーリングキュー（Full Runtime のみ）         │
│  ├── メモリアロケータ（jemalloc / mimalloc）      │
│  └── リフレクションメタデータ（.reflect セグメント、必要時に mmap）    │
│                                              │
│  なし：                                      │
│  ❌ バイトコードインタプリタ                             │
│  ❌ JIT コンパイラ                               │
│  ❌ GC                                      │
│  ❌ 仮想マシン                                    │
└────────────────────────────────────────────┘
```

**重要な設計**：コンパイル時に spawn ブロックのタスク識別と依存分析が完了し、ランタイムは「タスク作成 → スレッドプールに分发 → 完了待機」のみを行う——データ構造は固定、動作は予測可能。

> **RFC-008 のサイズ推定との差異**：RFC-008
> §4 はスケジューラは約 200-500KB と推定し、タスクスケジューリングコアのみを含む。本文書の 500KB-1MB 推定にはメモリアロケータ（jemalloc/mimalloc）、イベントループ（libuv/io_uring）、リフレクションメタデータセグメントが含まれている。実際のサイズはプラットフォームと機能選択に依存し、実装段階で正確な数字が提示される。

**三层ランタイムと LLVM の関係**（RFC-008 §1 に整合）：

| ランタイム   | LLVM AOT 動作                                                                                                             |
| ------------ | ------------------------------------------------------------------------------------------------------------------------- |
| **Embedded** | spawn サポートなし、直接順序機械語を生成                                                                                  |
| **Standard** | spawn ブロックをサポート、spawn ブロック内 DAG + 単一スレッドスケジューリング（num_workers=1）                            |
| **Full**     | spawn ブロックをサポート、spawn ブロック内 DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealing をサポート |

---

## 詳細な設計

### モジュールディレクトリ構造

[RFC-008](../accepted/008-runtime-concurrency-model.md)
§6 のディレクトリレイアウトに整合。`[! 計画中]`
マークはそのファイル/ディレクトリがまだ作成されていないことを示し、本 RFC の実装段階で導入される。

```
src/
├── frontend/                          # コンパイルフロントエンド（すべてのバックエンドで共有）
│   ├── core/
│   │   ├── spawn/                     # spawn モジュジール（VM と LLVM バックエンドで共有する並行分析）
│   │   │   ├── mod.rs                 # spawn モジュール入口
│   │   │   ├── placement.rs           # spawn 出現位置合法性チェック
│   │   │   └── analysis.rs            # [! 計画中] タスク識別、依存分析、リソース競合検出
│   │   └── typecheck/
│   │       └── ...
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR 定義（VM と LLVM で共用）
│   │   └── ir_gen.rs                  # IR 生成
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs                 # コラーション層（現在 BytecodeFile を出力）
│       │   ├── translator.rs          # IR → バイトコード翻訳（VM バックエンド用）
│       │   ├── emitter.rs             # バイトコード発行 + ジャンプバックフィル（VM バックエンド用）
│       │   ├── buffer.rs              # 定数プール + バイトコードバッファ（VM バックエンド用）
│       │   ├── bytecode.rs            # バイトコードフォーマット定義 + シリアライズ（VM バックエンド用）
│       │   ├── flow.rs                # レジスタ割り当て + ラベル生成 + シンボルテーブル（VM バックエンド用）
│       │   └── operand.rs             # オペランド解析（VM バックエンド用）
│       ├── lifetime/                  # ライフタイム/トークン活性分析
│       └── mono/                      # モノモン菲ゼーション
│
├── backends/
│   ├── common/                        # 共有値/ヒープ/オペコード
│   ├── interpreter/                   # 木たどри解釈器（VM バックエンド）
│   ├── llvm/                          # [! 計画中] LLVM バックエンドコード生成（下記ファイルリスト参照）
│   │   ├── mod.rs                     # [! 計画中] LLVM バックエンド入口
│   │   ├── context.rs                 # [! 計画中] LLVM コンテキスト管理
│   │   ├── types.rs                   # [! 計画中] 型マッピング（YaoXiang → LLVM IR）
│   │   ├── values.rs                  # [! 計画中] 値マッピング
│   │   ├── func.rs                    # [! 計画中] 関数翻訳
│   │   ├── spawn.rs                   # [! 計画中] spawn ブロック展開
│   │   ├── ffi.rs                     # [! 計画中] FFI 呼び出しコード生成
│   │   └── drop.rs                    # [! 計画中] デストラクタ挿入
│   └── runtime/                       # コンパイル時ランタイム（静的ライブラリとして exe にリンク）
│       ├── engine.rs                  # タスクスケジューリングエンジン
│       ├── facade.rs                  # 外部インターフェース
│       └── task.rs                    # タスク表現
│
└── util/
    └── diagnostic/                    # エラー診断（共有）
```

> **重要な変更**：spawn ブロック分析（タスク識別、依存分析、リソース競合検出）は
> `frontend/core/spawn/`（フロントエンド共有）で実装される。既存の
> `frontend/core/typecheck/passes/spawn_placement.rs`（spawn 出現位置チェック）は
> `frontend/core/spawn/placement.rs`
> に移行され、詳細については RFC-024 を参照。LLVM バックエンドは分析結果のみを消費し、対応するスケジューリングコードを生成する。
>
> **现状の説明**：現在の `middle/passes/codegen/` 下の
> `buffer.rs`、`emitter.rs`、`bytecode.rs`、`flow.rs`、`operand.rs`
> は VM バックエンドのバイトコード生成（`CodegenContext::generate()` →
> `BytecodeFile`）を提供する。LLVM バックエンドは `backends/llvm/`
> で実装され、interpreter バックエンドおよび runtime と同じレベル——両者は同一の `ModuleIR`
> 入力を共有し、異なるターゲットフォーマット（バイトコード vs ネイティブコード）を出力する。

### プラットフォーム ABI サポート

| プラットフォーム | ターゲットトリプレット     | 出力フォーマット | 呼び出し規約（FFI デフォルト） |
| ---------------- | -------------------------- | ---------------- | ------------------------------ |
| Linux x86_64     | `x86_64-unknown-linux-gnu` | ELF              | System V AMD64                 |
| macOS x86_64     | `x86_64-apple-darwin`      | Mach-O           | System V AMD64                 |
| macOS ARM64      | `aarch64-apple-darwin`     | Mach-O           | ARM64 AAPCS                    |
| Windows x86_64   | `x86_64-pc-windows-msvc`   | COFF             | Microsoft x64                  |

FFI 呼び出しはデフォルトでプラットフォームの C 呼び出し規約を使用する。ユーザーは
`native("symbol", cc = "stdcall")` などのオプションで上書きできる（後の拡張について
[RFC-026](./026-ffi-core-mechanism.md) に整合）。

### 浮動小数点意味論の一貫性（VM ↔ LLVM）

双方向バックエンドアーキテクチャのコア約束は、VM（開発デバッグ）と LLVM（本番リリース）の動作が一致することである。浮動小数点演算は 2 つの実行モード間で潜在的な不一致点が存在する：

| シナリオ         | リスク                                                                | 戦略                                                                                 |
| ---------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| NaN 伝播         | VM と LLVM が NaN の符号ビットと payload の處理が異なる可能性         | コンパイラは IR レベルで NaN 表現を正規化、NaN 比較は一貫して `fcmp uno` を使用      |
| 丸めモード       | LLVM はデフォルトで round-to-nearest-even、VM はホスト CPU に依存     | 非デフォルト丸めモードは公開しない、VM と LLVM は統一して RTNE を使用                |
| ゼロ除算         | IEEE 754 は ±Inf を定義するが、特定プラットフォームでは trap の可能性 | debug モードはゼロ除算をチェックして診断をレポート；release モードは IEEE 754 に従う |
| `-0.0` vs `+0.0` | 比較演算が同等でない可能性                                            | IEEE 754 ルールを一貫して使用：`+0.0 == -0.0`                                        |
| 非正規化数       | 特定プラットフォームでは flush-to-zero                                | LLVM は `denormal-fp-math` 属性を有効にしない、完全な IEEE 754 意味論を保持          |

> **テスト戦略**：跨后端の浮動小数点整合性テストスイートを実装——同じ YaoXiang ソースコードを VM と LLVM バックエンドでそれぞれ実行し、出力を逐値比較する。このテスト群は CI の強制門番となる。

---

## トレードオフ

### メリット

1. **性能**：AOT コンパイルは解釈実行より 10-100 倍高速
2. **統一フロントエンド**：VM と LLVM が同一のフロントエンドを共有し、動作が完全に一致
3. **ゼロスケジューリングオーバーヘッド**：通常コードは直接順序機械語を生成し、spawn ブロック外に DAG オーバーヘッドなし
4. **静的リンク**：外部ランタイム依存がなく、単一 exe でデプロイ可能
5. **ゼロ GC**：RAII 決定性破棄、 pausa なし
6. **FFI ゼロオーバーヘッド**：`?T`
   null ポインタ最適化、不透明型レイアウト最適化、FFI 呼び出しコストは C と同等
7. **コンパイル時分析**：spawn ブロックタスク識別と依存分析がコンパイル時に完了し、ランタイムは実行のみ

### デメリット

1. **LLVM 統合の複雑さ**：inkwell API と LLVM IR の深い理解が必要
2. **コンパイル時間**：AOT コンパイルはインタプリタより遅い（一度限りのコスト）
3. **デバッグ体験**：ネイティブコードデバッグには DWARF/PDB シンボルサポートが必要（コンパイラがデバッグ情報を生成する必要がある）
4. **增量コンパイル**：大型プロジェクトの增量コンパイルには追加設計が必要
5. **浮動小数点意味論の整合性**：VM と LLVM の NaN 伝播、丸めモード、ゼロ除算などのエッジ動作に差異が生じる可能性があり、正規化戦略を通じて双方向バックエンドの動作整合性を保証する必要がある（§10 参照）

### 関連 RFC との整合性

| RFC                              | 整合性                                                                               |
| -------------------------------- | ------------------------------------------------------------------------------------ |
| RFC-024 spawn ブロック並行モデル | ✅ spawn ブロック直接部分式 → タスク分发                                             |
| RFC-008 ランタイムアーキテクチャ | ✅ 双方向バックエンド + スケジューラ静的ライブラリ + モジュールディレクトリ構造      |
| RFC-009 所有権モデル v9          | ✅ `&T`/`&mut T` トークン（ゼロサイズ）、`ref T`（ファットポインタ）、`?T`（Option） |
| RFC-026 FFI コア機構             | ✅ `native()` → declare + marshalling、`.drop` → RAII cleanup                        |

---

## 代替案

| 方案                                 | 記述                       | 为什么不選                                                     |
| ------------------------------------ | -------------------------- | -------------------------------------------------------------- |
| インタプリタのみ                     | AOT が不要                 | 性能不足                                                       |
| 完全静的コンパイル（ランタイムなし） | スケジューラをリンクしない | spawn ブロックにはランタイムタスクスケジューリングが必要       |
| Cranelift バックエンド               | より高速なコンパイル速度   | ランタイム性能は LLVM に及ばない、未来的なオプション後端として |
| 外部 LLVM ランタイムをリンク         | LLVM 内蔵ランタイムを使用  | 不要な依存を導入                                               |

---

## 実装戦略

### 段階的分離

#### 段階 1：基礎フレームワーク

- [ ] inkwell 依存関係を追加
- [ ] LLVM コンテキスト初期化を実装（`context.rs`）
- [ ] 基礎型マッピングを実装（`types.rs`）

#### 段階 2：関数翻訳

- [ ] 関数宣言翻訳を実装（`func.rs`）
- [ ] 基礎命令翻訳を実装（算術、制御フロー、呼び出し）（`translator.rs`）
- [ ] 値マッピングを実装（`values.rs`）

#### 段階 3：所有権型翻訳

- [ ] `&T`/`&mut T` トークンを実装（ゼロサイズ、コンパイル後に消失）
- [ ] `ref T` を実装（ファットポインタ `{ i64*, T* }`）
- [ ] `?T` を実装（`{ i1, T }` tagged union）
- [ ] `List(T)` を実装（`{ T*, i64, i64 }`）
- [ ] Move 意味論追踪を実装（破棄挿入判断に使用）

#### 段階 4：spawn ブロックコード生成

- [ ] `spawn_placement.rs` の分析結果を消費
- [ ] 直接部分式 → タスク関数生成
- [ ] 依存タスクスケジューリングコード生成
- [ ] リソース競合直列化
- [ ] spawn for 展開

#### 段階 5：FFI コード生成

- [ ] `native()` → `declare external`（`ffi.rs`）
- [ ] パラメータ marshalling / 戻り値 unmarshalling
- [ ] 不透明型レイアウト（単フィールド最適化を含む）
- [ ] `?T` null ポインタ最適化（FFI 専用）

#### 段階 6：デストラクタコード生成

- [ ] `.drop` バインディング識別
- [ ] スコープ終了 cleanup 挿入（逆順）（`drop.rs`）
- [ ] 早期リターンパス cleanup
- [ ] `?` エラー伝播パス cleanup

#### 段階 7：ランタイムライブラリリンク

- [ ] `runtime_task_spawn` / `runtime_task_wait_all` などのランタイム関数を実装
- [ ] ランタイム静的ライブラリをリンク
- [ ] エンドツーエンド統合テスト

### 依存関係

- RFC-024（spawn ブロック並行）→ 段階 4 の入力
- RFC-009 v9（所有権）→ 段階 3、6 の入力
- RFC-008（ランタイムアーキテクチャ）→ 段階 7 の入力
- RFC-026（FFI 機構）→ 段階 5 の入力

---

## 関連作業

### Lazy Task Creation (1990)[^1]

| 属性     | 説明                                                   |
| -------- | ------------------------------------------------------ |
| 機関     | MIT                                                    |
| 著者     | James R. Larus, Robert H. Halstead Jr.                 |
| コア     | 遅延生成子タスク、需給で作成                           |
| 参考価値 | spawn ブロック内タスク需給スケジューリングの理論的基盤 |

**コア思想**：タスクを即座に作成するのではなく、遅延して作成する。親タスクが子タスクの値を必要とする时才创建子タスク。これにより細粒度并行任务的性能开销問題が解決される[^1]。YaoXiang の spawn ブロックスケジューリングはこの思想を借りている——タスクはコンパイル時に識別されるが、需給でスレッドプールに分发される。

### Lazy Scheduling (2014)[^2]

| 属性     | 説明                                           |
| -------- | ---------------------------------------------- |
| 機関     | University of Maryland                         |
| 著者     | Tzannes, Caragea                               |
| コア     | ランタイム適応スケジューリング、追加状態なし   |
| 参考価値 | Full Runtime WorkStealing スケジューラ設計参照 |

### SISAL 言語[^3]

| 属性     | 説明                                          |
| -------- | --------------------------------------------- |
| 機関     | Lawrence Livermore National Laboratory (LLNL) |
| コア     | 単一代入言語、Dataflow グラフ、暗黙的並列     |
| 参考価値 | Dataflow モデルの産業級応用の実行可能性証明   |

**重要な区別**：SISAL の並列性は**暗黙的**である——言語は単一代入意味論を持ち、コンパイラが全プログラムのデータ依存グラフを自動分析して並列性を決定する。YaoXiang の並列性は**明示的**である——ユーザーは
`spawn {}`
ブロックで並列領域をマークし、コンパイラは spawn ブロック内でのみ依存関係を分析する。これにより SISAL の全プログラム分析の複雑さを避けながら、ユーザーが並列動作を制御できる。

### Mul-T 並列 Scheme[^4]

| 属性     | 説明                                 |
| -------- | ------------------------------------ |
| 機関     | MIT                                  |
| コア     | Future 構成、Lazy Task Creation 実装 |
| 参考価値 | 具体的な実装参照                     |

### 比較まとめ

| 技術                   | 遅延作成 | 並列マーク                 | 分析範囲             | 所有権                          |
| ---------------------- | -------- | -------------------------- | -------------------- | ------------------------------- |
| Lazy Task Creation[^1] | ✅       | 暗黙                       | 全プログラム         | N/A                             |
| Lazy Scheduling[^2]    | ✅       | 暗黙                       | 全プログラム         | N/A                             |
| SISAL[^3]              | ✅       | 暗黙（単一代入）           | 全プログラム         | N/A                             |
| Mul-T[^4]              | ✅       | 明示（future）             | 呼び出し点           | N/A                             |
| **YaoXiang**           | ✅       | **明示（spawn ブロック）** | **spawn ブロック内** | **✅（Move + トークン + ref）** |

**YaoXiang の革新**：並列マークを「各関数呼び出し」（future）から「構造化ブロック」（spawn）に引き上げた。ユーザーは通常コードを書き、並列が必要な場所に spawn ブロックを置く。分析範囲は spawn ブロック内に制約され、コンパイルが効率的で動作が制御可能。

---

## 付録

### 付録 A：Rust async との比較

| 特性             | Rust async                   | YaoXiang LLVM AOT                                      |
| ---------------- | ---------------------------- | ------------------------------------------------------ |
| コンパイル生成物 | 状態機械 + 機械語            | 機械語 + spawn タスクメタデータ                        |
| ランタイム       | tokio                        | 静的リンクスケジューラ（约 500KB-1MB）                 |
| 並行マーク       | async/await キーワード       | `spawn { }` ブロック                                   |
| タスク作成       | コンパイル時に状態機械を生成 | コンパイル時に直接部分式を識別 → タスク関数            |
| コルーチン染色   | async 传染                   | **関数染色なし**                                       |
| 同期待機         | `.await`                     | spawn ブロックが自動的に同期ブロッキング               |
| メモリ管理       | GC（ランタイム）             | **RAII（决定的）**                                     |
| 共有機構         | `Arc::new()` + 手動 Weak     | **`ref` キーワード（コンパイラが Rc/Arc を自動選択）** |

### 付録 B：設計決定記録

| 決定                     | 決定                                                                                     | 日付       |
| ------------------------ | ---------------------------------------------------------------------------------------- | ---------- |
| LLVM AOT を採用          | 直接 Codegen、過度な抽象なし                                                             | 2026-02-15 |
| 並行モデル整合           | RFC-024 spawn ブロック直接部分式モデルに整合                                             | 2026-06-10 |
| DAG 分析範囲             | spawn ブロック内、spawn ブロック間をまたがらない（RFC-024 に整合）                       | 2026-06-05 |
| 所有権モデル整合         | RFC-009 v9 に整合：`&T`/`&mut T` トークン + `ref` キーワード                             | 2026-06-10 |
| 双方向バックエンドモデル | VM（開発）+ LLVM（本番）、RFC-008 に整合                                                 | 2026-05-11 |
| スケジューラ形態         | 静的ライブラリとして exe にリンク、約 500KB-1MB（プラットフォームと機能に依存）、GC なし | 2026-05-11 |
| FFI コード生成           | RFC-026 を統合：`native()` declare + marshalling                                         | 2026-06-10 |
| デストラクタ関数         | `.drop` → RAII cleanup 挿入、RFC-026 §7 に整合                                           | 2026-06-10 |
| 副作用処理               | `@IO`/`@Pure` 推論を削除、RFC-024 リソース型に変更                                       | 2026-06-10 |
| リフレクションメタデータ | exe .reflect セグメントにコンパイル、必要時に mmap でロード                              | 2026-05-11 |
| 論文引用                 | Lazy Task Creation などを保持し、YaoXiang の区別を明確化                                 | 2026-02-16 |

---

## 参考文献

[^1]:
    Larus, J. R., & Halstead, R. H. (1990). _Lazy Task Creation: A Technique for Increasing the
    Granularity of Parallel Programs_. MIT.

[^2]:
    Tzannes, A., & Caragea, G. (2014). _Lazy Scheduling: A Runtime Adaptive Scheduler for
    Declarative Parallelism_. University of Maryland.

[^3]:
    Feo, J. T., et al. (1990). _A report on the SISAL language project_. Lawrence Livermore National
    Laboratory.

[^4]: Mohr, E., et al. (1991). _Mul-T: A high-performance parallel lisp_. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024：spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 並行モデルとスケジューラの分離設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
- [RFC-026：FFI コア機構](./026-ffi-core-mechanism.md)

---

## ライフサイクルと行き先

| 状態         | 位置                        | 説明                                   |
| ------------ | --------------------------- | -------------------------------------- |
| **草案**     | `docs/design/rfc/`          | 著者草案、審查提出待ち                 |
| **審査中**   | `docs/design/rfc/review/`   | コミュニティ議論とフィードバックを開放 |
| **受入済み** | `docs/design/rfc/accepted/` | 正式設計文書になる                     |
| **却下**     | `docs/design/rfc/`          | RFC ディレクトリに保持                 |

> 現在の状態：**受入済み** — RFC-024 spawn ブロック並行モデル、RFC-009 v9 所有権モデル、RFC-026
> FFI 機構に整合済み
