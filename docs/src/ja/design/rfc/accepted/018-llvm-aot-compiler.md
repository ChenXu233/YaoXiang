---
title: 'RFC-018: LLVM AOT コンパイラ設計'
status: '承認済み'
author: '晨煦'
created: '2026-02-15'
updated: '2026-07-05（GitHub Issue #14、#134を同期；実装状態分析を追加）'
issue: '#14'
tracking_issue: 'https://github.com/ChenXu233/YaoXiang/issues/134'
---

# RFC-018: LLVM AOT コンパイラ設計

> **参考**:
>
> - [RFC-024: spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime並行モデルとスケジューラ疎結合設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md)
> - [RFC-026: FFIコア機構](./026-ffi-core-mechanism.md)
> - [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md)

> **廃止**:
>
> - 旧版「ボトムアップ自動DAG分析」モデル — RFC-024のspawnブロック直接子表現モデルに置き換え
> - `@IO`/`@Pure` 暗黙の副作用推論 — RFC-024のリソース型機構に置き換え
> - `Arc(T)` 型マッピング — RFC-009 v9の`ref`キーワードに置き換え

## 概要

本文書はYaoXiang言語のLLVM
AOT（事前コンパイル：Ahead-of-Time）コンパイラを設計する。LLVMバックエンドとVMバックエンド（インタプリタ）は同一のコンパイルフロントエンドを共有し、[RFC-008](../accepted/008-runtime-concurrency-model.md)
で定義されるデュアルバックエンドアーキテクチャを構成する：VMは開発デバッグ用、LLVMは本番リリース用。

**コア責務**:

```
ソースコード → フロントエンド（共有）→ IR → LLVM Codegen → .o → スケジューラ静的ライブラリをリンク → exe
```

コンパイラはYaoXiangソースコードをネイティブマシンコードにコンパイルする。

| 言語機能                 | コンパイル戦略                                                                                      |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| 通常コード               | 逐次マシンコード、スケジューリングオーバーヘッドゼロ                                                |
| `spawn { }` ブロック     | 直接子表現 → タスクディスパッチ + 同期待機（[RFC-024](../accepted/024-concurrency-model.md)に整合） |
| `native("symbol")`       | LLVM `declare external` + 引数マーシャリング（[RFC-026](./026-ffi-core-mechanism.md)に整合）        |
| `.drop` 析構             | RAIIクリーンアップコード挿入（[RFC-009](../accepted/009-ownership-model.md)に整合）                 |
| `&T` / `&mut T` トークン | ゼロサイズ型、コンパイル後消失                                                                      |
| `ref T` 共有             | `{ refcount_ptr, data_ptr }` ファットポインタ、コンパイラが自動的にRc/Arcを選択                     |

**RFC-024との関係**:
RFC-024はspawnブロックの**ユーザセマンティクス**（直接子表現によるタスク作成、同期ブロック待機）を定義する。本文書はこれらのセマンティクスを**マシンコードへコンパイルする方法**を定義する。

**RFC-026との関係**:
RFC-026はFFIの**ユーザ構文**（`native()`、`[0]`メソッドバインド、`.drop`）を定義する。本文書はFFI呼び出しが**LLVM
IRを生成する方法**を定義する。

---

## 動機

### なぜLLVM AOTコンパイラが必要か？

現在、YaoXiangには実行バックエンドとしてインタプリタのみが存在する：

| 問題             | 影響                                             |
| ---------------- | ------------------------------------------------ |
| 性能ボトルネック | インタプリタ実行はマシンコードより10〜100倍遅い  |
| デプロイの複雑さ | インタプリタとランタイムの同梱が必要             |
| 本番環境         | インタプリタは性能が要求されるシナリオに適さない |

### デュアルバックエンドモデルにおけるLLVM

[RFC-008](../accepted/008-runtime-concurrency-model.md)
§6はデュアルバックエンドアーキテクチャを定義する：

```
                    ┌─────────────────────┐
                    │   コンパイルフロントエンド（統一）     │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn分析       │
                    │   → エスケープ分析          │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VMバックエンド（開発）   │     │  LLVMバックエンド（本番）  │
      │   IR → インタプリタ実行    │     │  IR → ネイティブコード      │
      │   ステップデバッグ         │     │  スケジューラ静的ライブラリをリンク   │
      │   高速イテレーション         │     │  .exeを出力         │
      └───────────────────┘     └───────────────────┘
```

両バックエンドの**動作は完全に一致**する—違いは実行方式のみ。同一のソースコード、同一の型検査、同一のspawn分析結果。

---

## 提案

### 1. コンパイラアーキテクチャ

LLVMバックエンドはコンパイルパイプラインの最終段階に位置し、フロントエンドからIRを受信してネイティブコードを生成する：

```
ソースコード
  → Lexer / Parser（frontend/core/）
  → TypeCheck + spawn分析（frontend/core/typecheck/）
  → IR生成（middle/core/ir_gen.rs）
  → LLVM Codegen（backends/llvm/）
      ├── 型マッピング：YaoXiang型 → LLVM IR型
      ├── 関数翻訳：IR命令 → LLVM IR命令
      ├── spawn展開：直接子表現 → タスク関数 + スケジューラ呼び出し
      ├── FFI展開：native()呼び出し → declare + マーシャリング
      └── 析構挿入：スコープ終了 → .drop()呼び出し
  → LLVM最適化 + ターゲットコード生成
  → ランタイム静的ライブラリをリンク → 実行ファイル
```

### 2. コンパイルフロー

```
フェーズ1: フロントエンド（VMバックエンドと共有）
  - 構文解析、型検査、spawnブロック分析、エスケープ分析
  - 出力：型注釈付きIR

フェーズ2: LLVM IR生成
  - 型マッピング、関数宣言、命令翻訳
  - 出力：LLVM Module

フェーズ3: LLVM最適化
  - 標準LLVM最適化パイプライン（O0/O1/O2/O3）
  - インライン化、定数畳み込み、デッドコード除去

フェーズ4: ターゲットコード生成
  - LLVM TargetMachine → .oファイル
  - プラットフォーム：Linux (ELF)、macOS (Mach-O)、Windows (COFF)

フェーズ5: リンク
  - ランタイム静的ライブラリ（スケジューラ、アロケータ）をリンク
  - 出力：実行ファイル
```

### 3. 型マッピング

#### 3.1 YaoXiang → LLVM IR型マッピング

| YaoXiang型   | LLVM IR型                     | 説明                                                      |
| ------------ | ----------------------------- | --------------------------------------------------------- |
| `Int`        | `i64`                         | デフォルト64ビット符号付き整数                            |
| `Int32`      | `i32`                         | 明示的な32ビット整数（主にFFI用）                         |
| `Float`      | `f64`                         | デフォルト64ビット浮動小数点                              |
| `Float32`    | `f32`                         | 明示的な32ビット浮動小数点（主にFFI用）                   |
| `Bool`       | `i1`                          | ブール値                                                  |
| `Char`       | `i32`                         | Unicodeコードポイント                                     |
| `String`     | `{ i8*, i64 }`                | ポインタ + バイト長                                       |
| `Void`       | `{}`                          | ゼロサイズ空型                                            |
| `&T`         | —                             | ゼロサイズトークン、コンパイル後消失、IRを生成しない      |
| `&mut T`     | —                             | ゼロサイズトークン、コンパイル後消失、IRを生成しない      |
| `ref T`      | `{ i64*, T* }`                | ファットポインタ（参照カウントポインタ + データポインタ） |
| `*T`         | `T*`                          | 生ポインタ                                                |
| `[T; N]`     | `[N x T]`                     | 固定長配列                                                |
| `List(T)`    | `{ T*, i64, i64 }`            | データポインタ + 長さ + 容量                              |
| 構造体       | 対応するLLVM struct           | フィールドは定義順にレイアウト                            |
| レコードenum | `{ i64, [max_payload_size] }` | タグ + 最大payloadのunion                                 |
| `?T`         | `{ i1, T }`                   | 値タグ + データ（汎用表現）                               |
| FFI不透明型  | `{ i8* }`                     | Cポインタのラッパ                                         |
| 関数ポインタ | `T (...)*`                    | 関数ポインタ型                                            |

> **`&T` / `&mut T` ゼロランタイムオーバーヘッド**：[RFC-009](../accepted/009-ownership-model.md)
> §2.7は、コンパイラが内部でトークンにブランド識別子（コンパイル時一意整数）を割り当て、単態化とインライン化後にブランドが完全に消失することを定義している—生成されるマシンコードにはトークンの痕跡が一切残らない。

#### 3.2 FFI引数型マッピング

[RFC-026](./026-ffi-core-mechanism.md) §2.2に整合し、LLVM IR列を追加：

| C型                  | YaoXiang型      | LLVM IR        | 説明                                       |
| -------------------- | --------------- | -------------- | ------------------------------------------ |
| `int`                | `Int32`         | `i32`          |                                            |
| `long`               | `Int64`         | `i64`          |                                            |
| `float`              | `Float32`       | `f32`          |                                            |
| `double`             | `Float64`       | `f64`          |                                            |
| `char`               | `Char`          | `i32`          | C char → YaoXiang Char（Unicode互換）      |
| `char*`              | `String`        | `{ i8*, i64 }` | マーシャリング：C string → YaoXiang String |
| `bool`               | `Bool`          | `i1`           |                                            |
| `size_t`             | `Uint`          | `i64`          |                                            |
| `void*`              | `*Void`         | `i8*`          |                                            |
| `struct T*`          | `T`（透明型）   | `T*`           | ポインタ渡し                               |
| `typedef struct T T` | `T`（不透明型） | `{ i8* }`      | Cポインタのラッパ                          |

### 4. IR正規化と命令翻訳

#### 4.0 IR正規化（スタック → レジスタ）

現在のIR（`src/middle/core/ir.rs`）はスタック操作命令（`Push`/`Pop`/`Dup`/`Swap`）を含むが、これらはバイトコードVM用に設計されている。LLVM
IRはSSA形式であり、スタック操作を受け付けない。

**処理戦略**：LLVMパスは命令翻訳の前に、軽量な正規化パスを通過する：

| スタック命令 | 正規化戦略                                              |
| ------------ | ------------------------------------------------------- |
| `Push(r)`    | `stack.push(r)`を記録、IRを生成しない                   |
| `Pop(r)`     | `r = stack.pop()`、`load`を生成（スタックスロットから） |
| `Dup`        | `stack.push(stack.top())`、IRを生成しない               |
| `Swap`       | スタックトップの2要素を交換、IRを生成しない             |

正規化後、すべてのオペランドがレジスタ/ローカル変数参照となり、スタック操作は完全に除去される。このパスは`translator.rs`
の最初のステップとして実行される。

> **なぜIR層でスタック命令を除去しないのか？**
> VMバックエンドはスタックセマンティクスを必要とするため。LLVM翻訳の入口で正規化することで、IRが両バックエンドで共有される—各バックエンドは自身のニーズに応じて同じIRを消費する。
>
> **前提条件**：IR生成フェーズはスタックバランスを保証する—すべての制御フローパスが同じプログラム点に到達する際にスタック深度が一貫している（VMバイトコードバックエンドも同じ前提に依存し、そうでないとバイトコード実行が失敗する）。正規化パスはこの前提をチェックしない；違反した場合LLVMバックエンドは未定義動作を生成する。

#### 4.1 命令翻訳表

以下は`Instruction`enumの各バリアントのLLVM IR翻訳戦略を列挙する。命令名は`src/middle/core/ir.rs`
と完全に一致する。

**算術命令**:

| IR命令                  | LLVM IR                                    | 説明                                     |
| ----------------------- | ------------------------------------------ | ---------------------------------------- |
| `Add { dst, lhs, rhs }` | `add`（整数）/ `fadd`（浮動小数点）        | 型に応じて整数または浮動小数点加算を選択 |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub`                             |                                          |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul`                             |                                          |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv`                   | 符号付き/符号なし/浮動小数点除算         |
| `Mod { dst, lhs, rhs }` | `srem` / `urem`                            | 符号付き/符号なし剰余                    |
| `Neg { dst, src }`      | `sub 0, src`（整数）/ `fneg`（浮動小数点） |                                          |

> **`Mod`行の改訂予告（2026-09-22注、RFC-011bに伴う）**：`srem` / `urem`は**切り捨て剰余**
> （符号は被除数に従う）であり、本行の説明欄「剰余」は用語の混用である。[RFC-011b](./011b-operator-overloading.md)
> は`%`
> の既定セマンティクスを**数学的剰余**（結果の符号は除数に従う、言語リファレンス優先度表「乗除剰余」が優先）と確認済みで、このセマンティクスが実装された後、本行マッピングは
> `srem` + 符号修正シーケンス（または`sdiv`+`mul`+`sub`合成）に変更が必要で、実装側（インタプリタ`checked_rem`、定数畳み込み、バイトコード`I64_REM`）も同期して変更する。

**ビット演算命令**:

| IR命令                  | LLVM IR | 説明         |
| ----------------------- | ------- | ------------ |
| `And { dst, lhs, rhs }` | `and`   |              |
| `Or { dst, lhs, rhs }`  | `or`    |              |
| `Xor { dst, lhs, rhs }` | `xor`   |              |
| `Shl { dst, lhs, rhs }` | `shl`   | 左シフト     |
| `Shr { dst, lhs, rhs }` | `lshr`  | 論理右シフト |
| `Sar { dst, lhs, rhs }` | `ashr`  | 算術右シフト |

**比較命令**:

| IR命令                 | LLVM IR                 | 説明 |
| ---------------------- | ----------------------- | ---- |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq`  |      |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one`  |      |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` |      |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` |      |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` |      |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` |      |

**制御フロー命令**:

| IR命令                  | LLVM IR                                     | 説明                 |
| ----------------------- | ------------------------------------------- | -------------------- |
| `Jmp(label)`            | `br label %L`                               | 無条件ジャンプ       |
| `JmpIf(cond, label)`    | `br i1 %cond, label %L, label %fallthrough` | 条件付きジャンプ     |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | 条件付きジャンプせず |
| `Ret(Some(v))`          | `ret T %v`                                  | 戻り値あり           |
| `Ret(None)`             | `ret void`                                  | 戻り値なし           |

**呼び出し命令**:

| IR命令                                     | LLVM IR                             | 説明                                    |
| ------------------------------------------ | ----------------------------------- | --------------------------------------- |
| `Call { dst, func, args }`                 | `%r = call T @func(...)`            | 静的呼び出し                            |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call`（関数ポインタ） | 仮想メソッド呼び出し、vtable検索による  |
| `CallDyn { dst, func, args }`              | `%r = call T %func(...)`            | 動的呼び出し（クロージャ/関数ポインタ） |
| `TailCall { func, args }`                  | `musttail call` / `tail call`       | 末尾呼び出し最適化                      |

**メモリ命令**:

| IR命令                                | LLVM IR                                                           | 説明                                                          |
| ------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------- |
| `Move { dst, src }`                   | —                                                                 | 正規化後はレジスタコピーに変わり、SSA構築で大部分が消去される |
| `Load { dst, src }`                   | `%v = load T, T* %src`                                            |                                                               |
| `Store { dst, src }`                  | `store T %src, T* %dst`                                           |                                                               |
| `Alloc { dst, size }`                 | `%p = alloca T`（スタック）/ `call @malloc`（ヒープにエスケープ） | エスケープ分析が割り当て位置を決定                            |
| `Free(ptr)`                           | `call @free(%ptr)`（ヒープ）/ —（スタック、自動回収）             |                                                               |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]`（スタック）/ `call @malloc`（ヒープ）       |                                                               |

**構造体/配列アクセス命令**:

| IR命令                                    | LLVM IR                                                | 説明                            |
| ----------------------------------------- | ------------------------------------------------------ | ------------------------------- |
| `LoadField { dst, src, field }`           | `%ptr = getelementptr T, T* %src, 0, field` + `load`   |                                 |
| `StoreField { dst, field, src }`          | `%ptr = getelementptr T, T* %dst, 0, field` + `store`  |                                 |
| `LoadIndex { dst, src, index }`           | `%ptr = getelementptr T, T* %src, 0, %index` + `load`  |                                 |
| `StoreIndex { dst, index, src }`          | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` |                                 |
| `CreateStruct { dst, type_name, fields }` | `insertvalue`チェーン                                  | フィールド順にLLVM structを構成 |

**型変換命令**:

| IR命令                           | LLVM IR                                                                                                     | 説明                                                        |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | ソース/ターゲット型の組み合わせに応じて適切なcast命令を選択 |
| `TypeTest(val, type)`            | —                                                                                                           | コンパイル時型テスト、型タグを比較する`icmp eq`を生成       |

**所有権と借用命令**:

| IR命令                         | LLVM IR                                              | 説明                                                             |
| ------------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------- |
| `Borrow { dst, src, mutable }` | —                                                    | **ゼロサイズトークン、コンパイル後完全消失**、IRを一切生成しない |
| `Release(val)`                 | —                                                    | **ゼロサイズトークン、コンパイル後完全消失**                     |
| `Move { dst, src }`            | —                                                    | 所有権の移動、正規化後はレジスタコピーに変わる                   |
| `Drop(val)`                    | `call void @T.drop(T* %val)`                         | 型の析構関数を呼び出す（§7参照）                                 |
| `ShareRef { dst, src }`        | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | コンパイラがスレッド間共有かどうかに応じて自動的にArc/Rcを選択   |
| `ArcNew { dst, src }`          | `call %T* @Arc_new(%src)`                            | 不可複製参照カウント = 1                                         |
| `ArcClone { dst, src }`        | `call %T* @Arc_clone(%src)`                          | 不可複製参照カウントを不可複製に増加                             |
| `ArcDrop(val)`                 | `call void @Arc_drop(%val)`                          | 不可複製に減少 + 条件付き解放                                    |

**並行命令**:

| IR命令                             | LLVM IR                              | 説明                                                        |
| ---------------------------------- | ------------------------------------ | ----------------------------------------------------------- |
| `Spawn { closures, plan, result }` | スケジューラ呼び出しシーケンスに展開 | 詳細は§5、ランタイム`task_spawn` + `task_wait_all`          |
| `Yield`                            | —                                    | AOTパス上のspawnブロックは同期待機するため、yield不要；無視 |

**unsafeブロックと生ポインタ命令**:

| IR命令                    | LLVM IR                                                      | 説明                                   |
| ------------------------- | ------------------------------------------------------------ | -------------------------------------- |
| `UnsafeBlockStart`        | —                                                            | **コンパイル時マーカ、IRを生成しない** |
| `UnsafeBlockEnd`          | —                                                            | **コンパイル時マーカ、IRを生成しない** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64`（またはポインタを直接コピー） |                                        |
| `PtrDeref { dst, src }`   | `%v = load T, T* %src`                                       |                                        |
| `PtrStore { dst, src }`   | `store T %src, T* %dst`                                      |                                        |
| `PtrLoad { dst, src }`    | `%v = load T, T* %src`                                       |                                        |

**文字列命令**:

| IR命令                              | LLVM IR                                     | 説明                                    |
| ----------------------------------- | ------------------------------------------- | --------------------------------------- |
| `StringLength { dst, src }`         | `%len = extractvalue { i8*, i64 } %src, 1`  | Stringは`{ ptr, len }`、長はフィールド1 |
| `StringConcat { dst, lhs, rhs }`    | `call String @yx_string_concat(%lhs, %rhs)` | ランタイムヘルパー関数                  |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32`                | 境界チェックを含む                      |
| `StringFromInt { dst, src }`        | `call String @yx_string_from_int(%src)`     | ランタイムヘルパー関数                  |
| `StringFromFloat { dst, src }`      | `call String @yx_string_from_f64(%src)`     | ランタイムヘルパー関数                  |

**クロージャ命令**:

| IR命令                                   | LLVM IR                                                                 | 説明                                |
| ---------------------------------------- | ----------------------------------------------------------------------- | ----------------------------------- |
| `MakeClosure { dst, func: String, env }` | クロージャ構造体割り当て + 関数ポインタ（関数名による検索）と環境の入力 | `{ fn_ptr, env_fields... }`         |
| `LoadUpvalue { dst, upvalue_idx }`       | `%v = extractvalue %env, upvalue_idx`                                   | クロージャ環境からupvalueを読み取り |
| `StoreUpvalue { src, upvalue_idx }`      | `%env = insertvalue %env, %src, upvalue_idx`                            | クロージャ環境にupvalueを書き込み   |
| `CloseUpvalue(val)`                      | スタック上のupvalueをヒープにコピー                                     |                                     |

**その他の命令**:

| IR命令                          | LLVM IR                                       | 説明                    |
| ------------------------------- | --------------------------------------------- | ----------------------- |
| `HeapAlloc { dst, type_id }`    | `call i8* @malloc(i64 size)` + 型タグ書き込み | ヒープ割り当て + 型情報 |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)`      | ランタイムヘルパー関数  |

> **注意**：`Push`/`Pop`/`Dup`/`Swap`は§4.0の正規化フェーズで除去されるため、翻訳表には現れない。`Borrow`/`Release`
> はゼロサイズのコンパイル時トークンであり、マシンコードを一切生成しない。

### 5. spawnブロックコード生成

[RFC-024](../accepted/024-concurrency-model.md)に整合し、spawnブロックのコンパイルは以下のステップに分かれる。

#### 5.1 セマンティクスの振り返り

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // 直接子表現 → タスク1
    t2 = fetch("url2"),   // 直接子表現 → タスク2
    return (t1, t2)       // 同期待機、結果を組み立て
}
```

**ルール**（RFC-024 §2.1）:

- spawnブロックの**直接子表現**（トップレベルをカンマで区切った文）は並列タスクを作成する
- ネストした`{}`内の式は直接子表現と見なされず、独立したタスクにならない
- spawnブロック全体は同期的にブロックし、すべてのタスクの完了を待ってから結果を返す

#### 5.2 コンパイルステップ

```
ステップ1: 直接子表現の識別
  spawnブロック本体を走査し、トップレベル文を収集

ステップ2: 依存分析
  各直接子表現について、前のタスクが生成した変数を参照しているかを分析
  依存なし → 即座に並列ディスパッチ可能
  依存あり → 依存タスクの完了を待つようキューイング

ステップ3: リソース競合検出（RFC-024 §2.5）
  同一のリソース型インスタンスが複数のタスクで使用されているか確認
  同一インスタンスで競合 → 逐次実行順序をマーク

ステップ4: タスク関数の生成
  各直接子表現に対して独立したLLVM関数（クロージャ）を生成

ステップ5: スケジューリングコードの生成
  ランタイムスケジューラのtask_spawn / task_waitを呼び出す

ステップ6: 結果の組み立て
  すべてのタスク出力を収集し、returnのタプルを組み立て
```

#### 5.3 LLVM IR生成パターン

```llvm
; spawnブロック入口
%task_count = 2
%tasks = alloca [2 x %TaskHandle]

; タスク1を作成: fetch("url1")
%task1_fn = @spawn_closure_1
call @runtime_task_spawn(%tasks[0], %task1_fn, ...)

; タスク2を作成: fetch("url2")
%task2_fn = @spawn_closure_2
call @runtime_task_spawn(%tasks[1], %task2_fn, ...)

; すべてのタスクを同期待機
call @runtime_task_wait_all(%tasks, %task_count)

; 戻り値を組み立て
%r1 = call @runtime_task_result(%tasks[0])
%r2 = call @runtime_task_result(%tasks[1])
ret { %r1, %r2 }
```

#### 5.4 依存タスク

```yaoxiang
result = spawn {
    data = fetch("url"),       // タスク1: 依存なし
    processed = parse(data),   // タスク2: タスク1のdataに依存
    return processed
}
```

コンパイラは`parse(data)`がタスク1が生成した`data`を参照していることを検出し、スケジューリングコード生成時に依存をマークする：

```llvm
; タスク2はタスク1への依存付きで作成
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 タスク0（fetch）完了に依存
```

#### 5.5 リソース型の自動逐次化

[RFC-024 §2.5](../accepted/024-concurrency-model.md)
で定義されたリソース型（`FilePath`、`HttpUrl`、`DBUrl`、`Console`
およびユーザ定義のリソース型）はspawnブロック内で自動的に逐次化される：

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // SqliteDb（リソース型）を使用
    r2 = db.exec("INSERT ...")    // 同一インスタンス → 自動的に逐次化
}
```

コンパイラは同一のリソースインスタンスが2つのタスクで使用されていることを検出し、逐次依存を生成する：

```llvm
; タスク2はタスク1に依存（同一リソースの自動逐次化）
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for データ並列

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

コンパイラはN個の独立したタスク（N = itemsの長さ）に展開し、最大同時実行数の制限を受ける。

### 6. FFIコード生成

> ⚠️ **依存関係の説明**: 本節で定義されるFFIコード生成**アーキテクチャ**（`native("x")` →
> `declare external @x` →マーシャリングラッパー関数 →
> call）は安定しており、RFC-026の構文変更には影響されない。具体的な引数マーシャリングルール表（§6.2）と不透明型のレイアウト（§6.3）はRFC-026の定義を参照する—RFC-026の
> `native()`
> 構文やマーシャリングルールが変更された場合、本文書の対応するマッピング表を更新するだけでアーキテクチャ層は影響を受けない。RFC-026の現状：**レビュー中**で、本文書と同じ
> `review/` ディレクトリにある。
>
> **承認の前提条件**: 本RFCの承認前に、RFC-026のうち本文書§6に関連する部分（`native()`
> 宣言構文、引数マーシャリングルール、不透明型`{ i8* }`レイアウト、`.drop`
> バインド規約）を先に凍結するか、026とともに承認する必要がある。さもないと§6.2/§6.3/§7のマッピング表が実装前に陳腐化する可能性がある。

[RFC-026](./026-ffi-core-mechanism.md)に整合し、本節はFFI呼び出しのLLVM IR生成戦略を定義する。

#### 6.1 native()関数宣言

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

LLVM IRへコンパイル：

```llvm
; 外部C関数を宣言
declare i8* @sqlite3_open(i8*)

; YaoXiangラッパー関数（マーシャリングを処理）
define { i8* } @__yx_sqlite3_open({ i8*, i64 } %filename) {
    ; マーシャリング: YaoXiang String → C string
    %c_str = extractvalue { i8*, i64 } %filename, 0
    ; C関数を呼び出す
    %raw = call i8* @sqlite3_open(i8* %c_str)
    ; アンマーシャリング: Cポインタ → 不透明型
    %result = insertvalue { i8* } undef, i8* %raw, 0
    ret { i8* } %result
}
```

**重要なポイント**:

- `native("sqlite3_open")` → `declare external @sqlite3_open`
- コンパイラが自動的にマーシャリングラッパー関数を生成
- ラッパー関数のシグネチャはYaoXiang型を使用し、内部でC型に変換する

#### 6.2 引数マーシャリング

| 方向                                     | 変換                                |
| ---------------------------------------- | ----------------------------------- |
| YaoXiang `String` → C `char*`            | `.ptr`フィールド抽出して渡す        |
| YaoXiang `Int32` → C `int`               | 直接渡す（`i32`）                   |
| YaoXiang `*Void` → C `void*`             | 直接渡す（`i8*`）                   |
| YaoXiang `T`（透明型） → C `struct T*`   | アドレスを取得して渡す              |
| YaoXiang `T`（不透明型） → C `struct T*` | `{ i8* }`内のポインタを抽出して渡す |

#### 6.3 不透明型のLLVMレイアウト

[RFC-026](./026-ffi-core-mechanism.md) §4.1で定義された不透明型：

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVMレイアウト: `{ i8* }` — Cポインタを含む構造体。

**レイアウト最適化**: 不透明型が単一の`handle: *Void`フィールドしか持たない場合、
`i8*`を直接使用するように最適化できる（外側structを省略）。最適化後のABIはCポインタと完全に一致し、マーシャリングオーバーヘッドがゼロ。コンパイラはデフォルトでこの最適化を有効にし、ユーザはそれを意識する必要がない。

#### 6.4 ?T null許容戻り値のLLVM表現

[RFC-026](./026-ffi-core-mechanism.md) §7.6で定義されたFFI null許容戻り値：

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

汎用LLVM表現: `{ i1, { i8* } }` — 値タグ + データ。

**FFI nullポインタ向けの最適化**: `?T`の`T`が不透明型（内部がポインタ）の場合、コンパイラは
**nullポインタ = None** 最適化を使用する：

```llvm
; 最適化後のLLVM表現: null許容ポインタを直接使用
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None、null以外 → Some(不透明型にラップ)
    ret i8* %raw
}
```

呼び出し側：

```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

この最適化により`?SqliteDb`のFFI呼び出しは**追加オーバーヘッドゼロ**となり、Cのnullチェックと完全に等価になる。

#### 6.5 yx-bindgen統合

[yx-bindgen](./026-ffi-core-mechanism.md)
§6で自動生成されたバインドファイルは、コンパイル時に通常のYaoXiangソースコードとして扱われる。コンパイラはコードがbindgen由来であることを認識する必要がない—`native()`
宣言と`unsafe {}`型定義の処理方法は完全に同一である。

### 7. 析構関数コード生成

[RFC-009](../accepted/009-ownership-model.md)のRAIIセマンティクスと
[RFC-026](./026-ffi-core-mechanism.md) §7の`.drop`規約に整合する。

#### 7.1 .dropバインド識別

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

コンパイラは`.drop`バインドを識別し、型メタデータに析構関数ポインタをマークする。

#### 7.2 スコープ終了時のクリーンアップ挿入

```
ユーザコード:
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← スコープ終了
}

コンパイラが挿入するクリーンアップ（逆順）:
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**挿入位置**:

- 通常のスコープ終了（`}`）
- 早期リターン（`return`前）
- `?` エラー伝播パス（`?`前）
- spawnブロック終了（タスク内変数の析構）

#### 7.3 Moveと析構

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move: 所有権がdb2に転送される
// dbは無効、ここではdbのdropを挿入しない
// ← スコープ終了: db2のdropのみ挿入
```

コンパイラはMoveセマンティクス（[RFC-009](../accepted/009-ownership-model.md)
§1）を追跡し、変数の最終的な保有者の位置にのみ析構呼び出しを挿入する。

#### 7.4 析構失敗の処理

```llvm
; debugモード: 析構の戻り値をチェック
%ret = call i32 @sqlite3_close(i8* %handle)
%ok = icmp eq i32 %ret, 0
br i1 %ok, label %done, label %panic
panic:
  call @__yx_panic("destructor failed")
  unreachable
done:
  ret void

; releaseモード: 戻り値を無視
call i32 @sqlite3_close(i8* %handle)
ret void
```

### 8. コンパイル生成物の構造

コンパイル生成物は以下の構成要素を含む（具体的なstruct定義は実装段階で決定）：

- **マシンコード**: LLVMがコンパイルしたオブジェクトファイル（`.o`）、すべての関数翻訳結果を含む
- **spawnメタデータ**: 各spawnブロックのタスク関数ポインタ、依存関係、リソース競合の逐次化ペア
- **FFIシンボルテーブル**: 外部Cシンボル参照（シンボル名 + 弱参照かどうか）
- **エントリポイントテーブル**: 実行ファイルのエントリ関数リスト
- **型情報**: 実行ファイルの.reflectセグメントに書き込まれるリフレクションメタデータ、ランタイムが必要に応じてmmap

### 9. ランタイムライブラリ

[RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md)
に整合し、ランタイムは**静的ライブラリ**として最終exeにリンクされる。

```
最終exeの内部構造：

┌────────────────────────────────────────────┐
│  ユーザコード（ネイティブマシンコード）                       │
│  ├── 通常関数（逐次実行）                    │
│  ├── spawnブロック展開（タスク関数 + スケジューラ呼び出し）     │
│  ├── FFIマーシャリングラッパー関数               │
│  └── RAII析構コード                          │
├────────────────────────────────────────────┤
│  ランタイム静的ライブラリ（約500KB〜1MB、プラットフォームと機能選択に依存）  │
│  ├── スレッドプール（num_workers）                  │
│  ├── イベントループ（libuv / io_uring）           │
│  ├── ワークスティーリングキュー（Full Runtimeのみ）         │
│  ├── メモリアロケータ（jemalloc / mimalloc）      │
│  └── リフレクションメタデータ（.reflectセグメント、必要に応じてmmap）    │
│                                              │
│  含まないもの:                                      │
│  ❌ バイトコードインタプリタ                             │
│  ❌ JITコンパイラ                               │
│  ❌ GC                                      │
│  ❌ 仮想マシン                                    │
└────────────────────────────────────────────┘
```

**重要な設計**: コンパイル時にspawnブロックのタスク識別と依存分析を完了し、ランタイムは「タスク作成 → スレッドプールへのディスパッチ → 完了待機」のみ実行する—データ構造は固定され、動作は予測可能。

> **RFC-008のサイズ推定との差異**: RFC-008
> §4はスケジューラを約200〜500KBと推定しており、タスクスケジューリングコアのみを含む。本文書の500KB〜1MBの推定は、メモリアロケータ（jemalloc/mimalloc）、イベントループ（libuv/io_uring）、リフレクションメタデータセグメントを追加で含む。実際のサイズはプラットフォームと機能選択に依存し、実装段階で正確な数値を示す。

**3層ランタイムとLLVMの関係**（RFC-008 §1に整合）：

| ランタイム   | LLVM AOT動作                                                                                                      |
| ------------ | ----------------------------------------------------------------------------------------------------------------- |
| **Embedded** | spawnサポートなし、逐次マシンコードを直接生成                                                                     |
| **Standard** | spawnブロックサポート、spawnブロック内DAG + シングルスレッドスケジューリング（num_workers=1）                     |
| **Full**     | spawnブロックサポート、spawnブロック内DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealingサポート |

---

## 詳細設計

### モジュールディレクトリ構造

[RFC-008](../accepted/008-runtime-concurrency-model.md)
§6のディレクトリレイアウトに整合。`[! 計画中]`
マーカは、そのファイル/ディレクトリがまだ作成されておらず、本RFCの実装段階で導入されることを示す。

```
src/
├── frontend/                          # コンパイルフロントエンド（すべてのバックエンドで共有）
│   ├── core/
│   │   ├── spawn/                     # spawnモジュール（VMとLLVMバックエンドで共有される並行分析）
│   │   │   ├── mod.rs                 # spawnモジュールエントリ
│   │   │   ├── placement.rs           # spawn出現位置の合法性チェック
│   │   │   └── analysis.rs            # [! 計画中] タスク識別、依存分析、リソース競合検出
│   │   └── typecheck/
│   │       └── ...
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR定義（VMとLLVMで共有）
│   │   └── ir_gen.rs                  # IR生成
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs                 # オーケストレーション層（現在はBytecodeFileを出力）
│       │   ├── translator.rs          # IR → バイトコード翻訳（VMバックエンド用）
│       │   ├── emitter.rs             # バイトコードエミット + ジャンプバックパッチ（VMバックエンド用）
│       │   ├── buffer.rs              # 定数プール + バイトコードバッファ（VMバックエンド用）
│       │   ├── bytecode.rs            # バイトコードフォーマット定義 + シリアライズ（VMバックエンド用）
│       │   ├── flow.rs                # レジスタ割り当て + ラベル生成 + シンボルテーブル（VMバックエンド用）
│       │   └── operand.rs             # オペランド解析（VMバックエンド用）
│       ├── lifetime/                  # ライフタイム/トークン活性分析
│       └── mono/                      # 単態化
│
├── backends/
│   ├── common/                        # 共有値/ヒープ/オペコード
│   ├── interpreter/                   # ツリーウォーキングインタプリタ（VMバックエンド）
│   ├── llvm/                          # [! 計画中] LLVMバックエンドコード生成（後述のファイルリスト参照）
│   │   ├── mod.rs                     # [! 計画中] LLVMバックエンドエントリ
│   │   ├── context.rs                 # [! 計画中] LLVMコンテキスト管理
│   │   ├── types.rs                   # [! 計画中] 型マッピング（YaoXiang → LLVM IR）
│   │   ├── values.rs                  # [! 計画中] 値マッピング
│   │   ├── func.rs                    # [! 計画中] 関数翻訳
│   │   ├── spawn.rs                   # [! 計画中] spawnブロック展開
│   │   ├── ffi.rs                     # [! 計画中] FFI呼び出しコード生成
│   │   └── drop.rs                    # [! 計画中] 析構関数挿入
│   └── runtime/                       # コンパイル型ランタイム（静的ライブラリとしてexeにリンク）
│       ├── engine.rs                  # タスクスケジューリングエンジン
│       ├── facade.rs                  # 外部インターフェース
│       └── task.rs                    # タスク表現
│
└── util/
    └── diagnostic/                    # エラー診断（共有）
```

> **重要な変更**: spawnブロック分析（タスク識別、依存分析、リソース競合検出）は
> `frontend/core/spawn/`（フロントエンド共有）で実装される。既存の
> `frontend/core/typecheck/passes/spawn_placement.rs`（spawn出現位置チェック）は
> `frontend/core/spawn/placement.rs`に移行する。詳細はRFC-024を参照。LLVMバックエンドは分析結果のみを消費し、対応するスケジューリングコードを生成する。
>
> **現状の説明**: 現在の`middle/passes/codegen/`下の
> `buffer.rs`、`emitter.rs`、`bytecode.rs`、`flow.rs`、`operand.rs`
> はVMバックエンドのバイトコード生成（`CodegenContext::generate()` →
> `BytecodeFile`）に供される。LLVMバックエンドは
> `backends/llvm/`に実装され、interpreterバックエンドとruntimeと同列となる—両者は同じ`ModuleIR`
> 入力を共有し、異なる出力フォーマット（バイトコード vs ネイティブコード）を生成する。

### プラットフォームABIサポート

| プラットフォーム | ターゲットトリプレット     | 出力フォーマット | 呼び出し規約（FFIデフォルト） |
| ---------------- | -------------------------- | ---------------- | ----------------------------- |
| Linux x86_64     | `x86_64-unknown-linux-gnu` | ELF              | System V AMD64                |
| macOS x86_64     | `x86_64-apple-darwin`      | Mach-O           | System V AMD64                |
| macOS ARM64      | `aarch64-apple-darwin`     | Mach-O           | ARM64 AAPCS                   |
| Windows x86_64   | `x86_64-pc-windows-msvc`   | COFF             | Microsoft x64                 |

FFI呼び出しはデフォルトでプラットフォームのC呼び出し規約を使用。ユーザは`native("symbol", cc = "stdcall")`などのオプションで上書きできる（[RFC-026](./026-ffi-core-mechanism.md)の将来の拡張に整合）。

### 浮動小数点セマンティクスの一貫性（VM ↔ LLVM）

デュアルバックエンドアーキテクチャの中心的約束は、VM（開発デバッグ）とLLVM（本番リリース）の動作が一貫していること。浮動小数点演算は2つの実行モード間で潜在的な不整合点が存在する：

| シナリオ         | リスク                                                                   | 戦略                                                                           |
| ---------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------ |
| NaN伝播          | VMとLLVMがNaNの符号ビットとpayload処理を異なる可能性がある               | コンパイラがIR層でNaN表現を正規化、NaN比較は統一して`fcmp uno`を使用           |
| 丸めモード       | LLVMはデフォルトでround-to-nearest-even、VMはホストCPUに依存             | デフォルト以外の丸めモードを公開しない、VMとLLVMは統一してRTNEを使用           |
| ゼロ除算         | IEEE 754は±Infを定義するが、一部のプラットフォームはtrapする可能性がある | debugモードでゼロ除算をチェックして診断を報告；releaseモードはIEEE 754に従う   |
| `-0.0` vs `+0.0` | 比較操作が等価でない可能性がある                                         | IEEE 754ルールを統一適用: `+0.0 == -0.0`                                       |
| 非正規化数       | 一部のプラットフォームはflush-to-zero                                    | LLVMは`denormal-fp-math`属性を有効にしない、完全なIEEE 754セマンティクスを保持 |

> **テスト戦略**: バックエンド横断の浮動小数点一貫性テストスイートを実装する—同じYaoXiangソースコードをVMとLLVMバックエンドでそれぞれ実行し、値ごとに出力を比較する。このテストスイートはCIの必須ゲート。

---

## トレードオフ

### 利点

1. **性能**: AOTコンパイルはインタプリタ実行より10〜100倍速い
2. **統一フロントエンド**: VMとLLVMが同一のフロントエンドを共有し、動作が完全に一致
3. **ゼロスケジューリングオーバーヘッド**: 通常コードは逐次マシンコードを直接生成、spawnブロック外にDAGオーバーヘッドなし
4. **静的リンク**: 外部ランタイム依存なし、単一のexeでデプロイ可能
5. **GCゼロ**: RAIIによる決定論的析構、ポーズなし
6. **FFIゼロオーバーヘッド**: `?T`
   nullポインタ最適化、不透明型レイアウト最適化、FFI呼び出しコストはCと同等
7. **コンパイル時分析**:
   spawnブロックのタスク識別と依存分析はコンパイル時に完了、ランタイムは実行のみ

### 欠点

1. **LLVM統合の複雑さ**: inkwell APIとLLVM IRの深い理解が必要
2. **コンパイル時間**: AOTコンパイルはインタプリタより遅い（一回限りのコスト）
3. **デバッグ体験**: ネイティブコードのデバッグにはDWARF/PDBシンボルサポートが必要（コンパイラがデバッグ情報を生成する必要がある）
4. **インクリメンタルコンパイル**: 大規模プロジェクトのインクリメンタルコンパイルには追加設計が必要
5. **浮動小数点セマンティクスの一貫性**:
   VMとLLVMがNaN伝播、丸めモード、ゼロ除算などの境界動作で差異が生じる可能性があり、正規化戦略で両バックエンドの動作一貫性を保証する必要がある（§10参照）

### 関連RFCとの一貫性

| RFC                              | 一貫性                                                                              |
| -------------------------------- | ----------------------------------------------------------------------------------- |
| RFC-024 spawnブロック並行モデル  | ✅ spawnブロック直接子表現 → タスクディスパッチ                                     |
| RFC-008 ランタイムアーキテクチャ | ✅ デュアルバックエンド + スケジューラ静的ライブラリ + モジュールディレクトリ構造   |
| RFC-009 所有権モデルv9           | ✅ `&T`/`&mut T`トークン（ゼロサイズ）、`ref T`（ファットポインタ）、`?T`（Option） |
| RFC-026 FFIコア機構              | ✅ `native()` → declare + マーシャリング、`.drop` → RAIIクリーンアップ              |

---

## 代替案

| 案                                     | 説明                       | 選ばない理由                                                         |
| -------------------------------------- | -------------------------- | -------------------------------------------------------------------- |
| インタプリタのみ                       | AOT不要                    | 性能不足                                                             |
| 純粋な静的コンパイル（ランタイムなし） | スケジューラをリンクしない | spawnブロックはランタイムタスクスケジューリングが必要                |
| Craneliftバックエンド                  | より高速なコンパイル速度   | ランタイム性能がLLVMに劣る、将来のオプションのバックエンドとして検討 |
| 外部LLVMランタイムをリンク             | LLVM内蔵ランタイムを使用   | 不要な依存関係の導入                                                 |

---

## 実装戦略

### フェーズ分割

#### フェーズ1: 基本フレームワーク

- [ ] inkwell依存関係を追加
- [ ] LLVMコンテキスト初期化を実装（`context.rs`）
- [ ] 基本型マッピングを実装（`types.rs`）

#### フェーズ2: 関数翻訳

- [ ] 関数宣言翻訳を実装（`func.rs`）
- [ ] 基本命令翻訳を実装（算術、制御フロー、呼び出し）（`translator.rs`）
- [ ] 値マッピングを実装（`values.rs`）

#### フェーズ3: 所有権型の翻訳

- [ ] `&T`/`&mut T`トークンを実装（ゼロサイズ、コンパイル後消失）
- [ ] `ref T`を実装（ファットポインタ`{ i64*, T* }`）
- [ ] `?T`を実装（`{ i1, T }`タグ付きユニオン）
- [ ] `List(T)`を実装（`{ T*, i64, i64 }`）
- [ ] Moveセマンティクス追跡を実装（析構挿入判定用）

#### フェーズ4: spawnブロックコード生成

- [ ] `spawn_placement.rs`の分析結果を消費
- [ ] 直接子表現 → タスク関数生成
- [ ] 依存タスクスケジューリングコード生成
- [ ] リソース競合の逐次化
- [ ] spawn forの展開

#### フェーズ5: FFIコード生成

- [ ] `native()` → `declare external`（`ffi.rs`）
- [ ] 引数マーシャリング / 戻り値アンマーシャリング
- [ ] 不透明型レイアウト（単一フィールド最適化を含む）
- [ ] `?T` nullポインタ最適化（FFI専用）

#### フェーズ6: 析構関数コード生成

- [ ] `.drop`バインド識別
- [ ] スコープ終了クリーンアップ挿入（逆順）（`drop.rs`）
- [ ] 早期リターンパスのクリーンアップ
- [ ] `?` エラー伝播パスのクリーンアップ

#### フェーズ7: ランタイムライブラリリンク

- [ ] `runtime_task_spawn` / `runtime_task_wait_all`などのランタイム関数を実装
- [ ] ランタイム静的ライブラリをリンク
- [ ] エンドツーエンド統合テスト

### 依存関係

- RFC-024（spawnブロック並行）→ フェーズ4への入力
- RFC-009 v9（所有権）→ フェーズ3、6への入力
- RFC-008（ランタイムアーキテクチャ）→ フェーズ7への入力
- RFC-026（FFI機構）→ フェーズ5への入力

---

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性     | 説明                                                        |
| -------- | ----------------------------------------------------------- |
| 機関     | MIT                                                         |
| 著者     | James R. Larus, Robert H. Halstead Jr.                      |
| コア     | 子タスクの遅延作成、需要に応じて作成                        |
| 参考価値 | spawnブロック内のオンデマンドタスクディスパッチの理論的基盤 |

**中心思想**: タスクを即座に作成するのではなく、遅延作成する。親タスクが子タスクの値を必要とした時にのみ子タスクを作成する。これは細粒度並列タスクの性能オーバーヘッド問題を解決する[^1]。YaoXiangのspawnブロックスケジューリングはこの思想を借用している—タスクはコンパイル時に識別されるが、ランタイムではオンデマンドでスレッドプールにディスパッチされる。

### Lazy Scheduling (2014)[^2]

| 属性     | 説明                                            |
| -------- | ----------------------------------------------- |
| 機関     | University of Maryland                          |
| 著者     | Tzannes, Caragea                                |
| コア     | ランタイム適応型スケジューリング、追加状態なし  |
| 参考価値 | Full Runtime WorkStealingスケジューラ設計の参考 |

### SISAL言語[^3]

| 属性     | 説明                                           |
| -------- | ---------------------------------------------- |
| 機関     | Lawrence Livermore National Laboratory (LLNL)  |
| コア     | 単一代入言語、Dataflowグラフ、暗黙の並列化     |
| 参考価値 | Dataflowモデルの産業レベル応用の実現可能性証明 |

**重要な違い**:
SISALの並列性は**暗黙的**である—言語は単一代入セマンティクスで、コンパイラが自動的にプログラム全体のデータ依存グラフを分析して並列性を決定する。YaoXiangの並列性は**明示的**である—ユーザは`spawn {}`
ブロックで並列領域をマークし、コンパイラはspawnブロック内でのみ依存を分析する。これはSISALの全プログラム分析の複雑さを回避しつつ、ユーザの並列動作に対する制御を保持する。

### Mul-T並列Scheme[^4]

| 属性     | 説明                                         |
| -------- | -------------------------------------------- |
| 機関     | MIT                                          |
| コア     | Futureコンストラクト、Lazy Task Creation実装 |
| 参考価値 | 具体的な実装参考                             |

### 比較まとめ

| 技術                   | 遅延作成 | 並列性マーク                | 分析範囲            | 所有権                          |
| ---------------------- | -------- | --------------------------- | ------------------- | ------------------------------- |
| Lazy Task Creation[^1] | ✅       | 暗黙的                      | プログラム全体      | N/A                             |
| Lazy Scheduling[^2]    | ✅       | 暗黙的                      | プログラム全体      | N/A                             |
| SISAL[^3]              | ✅       | 暗黙的（単一代入）          | プログラム全体      | N/A                             |
| Mul-T[^4]              | ✅       | 明示的（future）            | 呼び出し点          | N/A                             |
| **YaoXiang**           | ✅       | **明示的（spawnブロック）** | **spawnブロック内** | **✅（Move + トークン + ref）** |

**YaoXiangの革新**: 並列性マークを「各関数呼び出し」（future）から「構造化ブロック」（spawn）に昇格させ、ユーザは通常のコードを書き、並列化が必要な場所にspawnブロックを置く。分析範囲はspawnブロック内に制約され、コンパイルが効率的で動作が制御可能。

---

## 付録

### 付録A: Rust asyncとの比較

| 特性             | Rust async                       | YaoXiang LLVM AOT                                       |
| ---------------- | -------------------------------- | ------------------------------------------------------- |
| コンパイル生成物 | ステートマシン + マシンコード    | マシンコード + spawnタスクメタデータ                    |
| ランタイム       | tokio                            | スケジューラを静的リンク（約500KB〜1MB）                |
| 並行性マーク     | async/awaitキーワード            | `spawn { }` ブロック                                    |
| タスク作成       | コンパイル時にステートマシン生成 | コンパイル時に直接子表現を識別 → タスク関数             |
| 色付き関数       | async伝染                        | **関数色なし**                                          |
| 同期待機         | `.await`                         | spawnブロックは自動的に同期ブロック                     |
| メモリ管理       | GC（ランタイム）                 | **RAII（決定的）**                                      |
| 共有機構         | `Arc::new()` + 手動Weak          | **`ref`キーワード（コンパイラが自動的にRc/Arcを選択）** |

### 付録B: 設計決定の記録

| 決定                       | 決定内容                                                                              | 日付       |
| -------------------------- | ------------------------------------------------------------------------------------- | ---------- |
| LLVM AOTを採用             | 直接Codegen、過度な抽象化を避ける                                                     | 2026-02-15 |
| 並行モデルの整合性         | RFC-024 spawnブロック直接子表現モデルに整合                                           | 2026-06-10 |
| DAG分析範囲                | spawnブロック内、spawnブロックを跨がない（RFC-024に整合）                             | 2026-06-05 |
| 所有権モデルの整合性       | RFC-009 v9に整合: `&T`/`&mut T`トークン + `ref`キーワード                             | 2026-06-10 |
| デュアルバックエンドモデル | VM（開発）+ LLVM（本番）、RFC-008に整合                                               | 2026-05-11 |
| スケジューラ形態           | 静的ライブラリとしてexeにリンク、約500KB〜1MB（プラットフォームと機能に依存）、GCなし | 2026-05-11 |
| FFIコード生成              | RFC-026を統合: `native()` declare + マーシャリング                                    | 2026-06-10 |
| 析構関数                   | `.drop` → RAIIクリーンアップ挿入、RFC-026 §7に整合                                    | 2026-06-10 |
| 副作用処理                 | `@IO`/`@Pure`推論を削除、RFC-024のリソース型に変更                                    | 2026-06-10 |
| リフレクションメタデータ   | exeの.reflectセグメントにコンパイル、mmapでオンデマンドロード                         | 2026-05-11 |
| 論文引用                   | Lazy Task Creationなどを保持、YaoXiangの違いを明確化                                  | 2026-02-16 |

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
- [RFC-024: spawnブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime並行モデルとスケジューラ疎結合設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md)
- [RFC-026: FFIコア機構](./026-ffi-core-mechanism.md)

---

## ライフサイクルと帰趣

| 状態           | 位置                        | 説明                                       |
| -------------- | --------------------------- | ------------------------------------------ |
| **ドラフト**   | `docs/design/rfc/`          | 著者のドラフト、提出レビュー待ち           |
| **レビュー中** | `docs/design/rfc/review/`   | オープンなコミュニティ議論とフィードバック |
| **承認済み**   | `docs/design/rfc/accepted/` | 正式な設計文書になる                       |
| **却下**       | `docs/design/rfc/`          | RFCディレクトリに保持                      |

> 現在の状態: **承認済み** — RFC-024 spawnブロック並行モデル、RFC-009 v9所有権モデル、RFC-026
> FFI機構に整合済み
