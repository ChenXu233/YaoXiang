---
title: "RFC-018: LLVM AOT コンパイラ設計"
status: "承認済み"
author: "晨煦"
created: "2026-02-15"
updated: "2026-06-11（§3.1 ref T 型 {i32*→i64*}、§4.1 MakeClosure func フィールド型、§6 RFC-026 アーキテクチャとルール分離、§8 コンパイル成果物構造の具体化解除、§9 spawn モジュールディレクトリ再構築、浮動小数点セマンティクス一貫性セクション追加、ランタイムライブラリサイズ推定修正を修正；承認前に補足：§4.0 スタックバランス前提宣言、§6 RFC-026 凍結前提条件、§9 RFC-008 とのサイズ推定差異説明）"
---

# RFC-018: LLVM AOT コンパイラ設計

> **参考**:
> - [RFC-024: spawn ブロックベースの並行性モデル](../accepted/024-concurrency-model.md)
> - [RFC-008: Runtime 並行性モデルとスケジューラ疎結合設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md)
> - [RFC-026: FFI コアメカニズム](./026-ffi-core-mechanism.md)
> - [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md)

> **廃止**:
> - 旧版「ボトムアップ自動 DAG 分析」モデル — RFC-024 spawn ブロック直接子式モデルに置き換え
> - `@IO`/`@Pure` 暗黙の副作用推論 — RFC-024 リソース型メカニズムに置き換え
> - `Arc(T)` 型マッピング — RFC-009 v9 `ref` キーワードに置き換え

## 要約

本文書は YaoXiang 言語の LLVM AOT（Ahead-of-Time）コンパイラを設計する。LLVM バックエンドと VM バックエンド（インタプリタ）は同一のコンパイルフロントエンドを共有し、[RFC-008](../accepted/008-runtime-concurrency-model.md) で定義されるデュアルバックエンドアーキテクチャを構成する：VM は開発デバッグ用、LLVM は本番リリース用。

**コア責務**：

```
ソースコード → フロントエンド（共有）→ IR → LLVM Codegen → .o → スケジューラ静的ライブラリをリンク → exe
```

コンパイラは YaoXiang ソースコードをネイティブマシンコードにコンパイルする。

| 言語機能 | コンパイル戦略 |
|----------|----------|
| 通常コード | シーケンシャルマシンコード、ゼロスケジューリングオーバーヘッド |
| `spawn { }` ブロック | 直接子式 → タスクディスパッチ + 同期待機（[RFC-024](../accepted/024-concurrency-model.md) に整合） |
| `native("symbol")` | LLVM `declare external` + 引数マーシャリング（[RFC-026](./026-ffi-core-mechanism.md) に整合） |
| `.drop` デストラクタ | RAII cleanup コード挿入（[RFC-009](../accepted/009-ownership-model.md) に整合） |
| `&T` / `&mut T` トークン | ゼロサイズ型、コンパイル後消滅 |
| `ref T` 共有 | `{ refcount_ptr, data_ptr }` ファットポインタ、コンパイラが Rc/Arc を自動選択 |

**RFC-024 との関係**：RFC-024 は spawn ブロックの**ユーザセマンティクス**（直接子式によるタスク作成、同期ブロッキング待機）を定義する。本文書はこれらのセマンティクスが**マシンコードへどのようにコンパイルされるか**を定義する。

**RFC-026 との関係**：RFC-026 は FFI の**ユーザ構文**（`native()`、`[0]` メソッドバインド、`.drop`）を定義する。本文書は FFI 呼び出しが**LLVM IR をどのように生成するか**を定義する。

---

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在 YaoXiang は実行バックエンドとしてインタプリタのみを持つ：

| 問題 | 影響 |
|------|------|
| パフォーマンスボトルネック | 解釈実行はマシンコードより 10-100 倍遅い |
| デプロイの複雑さ | インタプリタとランタイムの携带が必要 |
| 本番環境 | インタプリタはパフォーマンス敏感なシナリオに適さない |

### デュアルバックエンドモデルにおける LLVM

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 はデュアルバックエンドアーキテクチャを定義する：

```
                    ┌─────────────────────┐
                    │  コンパイルフロントエンド（統一） │
                    │  Lexer → Parser     │
                    │  → TypeCheck        │
                    │  → spawn 分析       │
                    │  → エスケープ分析    │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │  VM バックエンド（開発）│     │ LLVM バックエンド（本番）│
      │  IR → 解釈実行    │     │  IR → ネイティブコード  │
      │  ステップデバッグ  │     │  スケジューラ静的ライブラリをリンク │
      │  高速イテレーション │     │  .exe を出力         │
      └───────────────────┘     └───────────────────┘
```

両バックエンドの**動作は完全に一致**する——違いは実行方法のみ。同じソースコード、同じ型チェック、同じ spawn 分析結果。

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
      ├── spawn 展開：直接子式 → タスク関数 + スケジューリング呼び出し
      ├── FFI 展開：native() 呼び出し → declare + マーシャリング
      └── デストラクタ挿入：スコープ終了 → .drop() 呼び出し
  → LLVM 最適化 + ターゲットコード生成
  → ランタイム静的ライブラリをリンク → 実行可能ファイル
```

### 2. コンパイルフロー

```
Phase 1: フロントエンド（VM バックエンドと共有）
  - 解析、型チェック、spawn ブロック分析、エスケープ分析
  - 出力：型注釈付き IR

Phase 2: LLVM IR 生成
  - 型マッピング、関数宣言、命令翻訳
  - 出力：LLVM Module

Phase 3: LLVM 最適化
  - 標準 LLVM 最適化パイプライン（O0/O1/O2/O3）
  - インライン化、定数畳み込み、デッドコード除去

Phase 4: ターゲットコード生成
  - LLVM TargetMachine → .o ファイル
  - プラットフォーム：Linux (ELF)、macOS (Mach-O)、Windows (COFF)

Phase 5: リンク
  - ランタイム静的ライブラリ（スケジューラ、アロケータ）をリンク
  - 出力：実行可能ファイル
```

### 3. 型マッピング

#### 3.1 YaoXiang → LLVM IR 型マッピング

| YaoXiang 型 | LLVM IR 型 | 説明 |
|---------------|-------------|------|
| `Int` | `i64` | デフォルト 64 ビット符号付き整数 |
| `Int32` | `i32` | 明示的 32 ビット整数（主に FFI 用） |
| `Float` | `f64` | デフォルト 64 ビット浮動小数点 |
| `Float32` | `f32` | 明示的 32 ビット浮動小数点（主に FFI 用） |
| `Bool` | `i1` | ブール値 |
| `Char` | `i32` | Unicode コードポイント |
| `String` | `{ i8*, i64 }` | ポインタ + バイト長 |
| `Void` | `{}` | ゼロサイズ空型 |
| `&T` | — | ゼロサイズトークン、コンパイル後消滅、IR を一切生成しない |
| `&mut T` | — | ゼロサイズトークン、コンパイル後消滅、IR を一切生成しない |
| `ref T` | `{ i64*, T* }` | ファットポインタ（参照カウントポインタ + データポインタ） |
| `*T` | `T*` | 生ポインタ |
| `[T; N]` | `[N x T]` | 固定長配列 |
| `List(T)` | `{ T*, i64, i64 }` | データポインタ + 長さ + 容量 |
| 構造体 | 対応する LLVM struct | フィールドは定義順にレイアウト |
| レコード enum | `{ i64, [max_payload_size] }` | タグ + 最大ペイロードの union |
| `?T` | `{ i1, T }` | 有値マーカー + データ（汎用表現） |
| FFI 不透明型 | `{ i8* }` | C ポインタをラップ |
| 関数ポインタ | `T (...)*` | 関数ポインタ型 |

> **`&T` / `&mut T` ゼロランタイムオーバーヘッド**：[RFC-009](../accepted/009-ownership-model.md) §2.7 は、コンパイラ内部でトークンにブランド識別子（コンパイル時一意整数）を割り当てることを定義しており、単態化とインライン化の後、ブランドは完全に消滅する——生成されるマシンコードにはトークンの痕跡が一切存在しない。

#### 3.2 FFI 引数型マッピング

[RFC-026](./026-ffi-core-mechanism.md) §2.2 に整合し、LLVM IR 列を補足：

| C 型 | YaoXiang 型 | LLVM IR | 説明 |
|--------|---------------|---------|------|
| `int` | `Int32` | `i32` | |
| `long` | `Int64` | `i64` | |
| `float` | `Float32` | `f32` | |
| `double` | `Float64` | `f64` | |
| `char` | `Char` | `i32` | C char → YaoXiang Char（Unicode 互換） |
| `char*` | `String` | `{ i8*, i64 }` | マーシャリング：C 文字列 → YaoXiang String |
| `bool` | `Bool` | `i1` | |
| `size_t` | `Uint` | `i64` | |
| `void*` | `*Void` | `i8*` | |
| `struct T*` | `T`（透過型） | `T*` | ポインタを渡す |
| `typedef struct T T` | `T`（不透明型） | `{ i8* }` | C ポインタをラップ |

### 4. IR 正規化と命令翻訳

#### 4.0 IR 正規化（スタック → レジスタ）

現在の IR（`src/middle/core/ir.rs`）はスタック操作命令（`Push`/`Pop`/`Dup`/`Swap`）を含むが、これはバイトコード VM 用に設計されている。LLVM IR は SSA 形式であり、スタック操作を受け付けない。

**処理戦略**：LLVM パスは命令翻訳の前に、まず軽量な正規化パスを通す：

| スタック命令 | 正規化戦略 |
|--------|-----------|
| `Push(r)` | `stack.push(r)` を記録、IR を生成しない |
| `Pop(r)` | `r = stack.pop()`、`load` を生成（スタックスロットから） |
| `Dup` | `stack.push(stack.top())`、IR を生成しない |
| `Swap` | スタックトップの 2 要素を交換、IR を生成しない |

正規化後、すべてのオペランドはレジスタ/局所変数参照となり、スタック操作は完全に除去される。このパスは `translator.rs` の最初のステップとして実行される。

> **なぜ IR 層でスタック命令を除去しないのか？** VM バックエンドはスタックセマンティクスを必要とする。LLVM 翻訳入口で正規化することで、IR の両バックエンド共有が維持される——各バックエンドは自身のニーズに応じて同じ IR を消費する。
>
> **前提**：IR 生成フェーズはスタックバランスを保証する——すべての制御フローパスが同一プログラム点に到達した時、スタック深さが一致する（VM バイトコードバックエンドも同一の前提に依存し、違反するとバイトコード実行がエラーになる）。正規化パスはこの前提をチェックしない；違反した場合 LLVM バックエンドは未定義動作を生成する。

#### 4.1 命令翻訳表

以下は `Instruction` enum の各バリアントに対する LLVM IR 翻訳戦略を逐一列挙する。命令名は `src/middle/core/ir.rs` と完全に一致する。

**算術命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Add { dst, lhs, rhs }` | `add`（整数）/ `fadd`（浮動小数点） | 型に応じて整数または浮動小数点加算を選択 |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub` | |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul` | |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` | 符号付き/符号なし/浮動小数点除算 |
| `Mod { dst, lhs, rhs }` | `srem` / `urem` | 符号付き/符号なし剰余 |
| `Neg { dst, src }` | `sub 0, src`（整数）/ `fneg`（浮動小数点） | |

**ビット演算命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `And { dst, lhs, rhs }` | `and` | |
| `Or { dst, lhs, rhs }` | `or` | |
| `Xor { dst, lhs, rhs }` | `xor` | |
| `Shl { dst, lhs, rhs }` | `shl` | 左シフト |
| `Shr { dst, lhs, rhs }` | `lshr` | 論理右シフト |
| `Sar { dst, lhs, rhs }` | `ashr` | 算術右シフト |

**比較命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq` | |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one` | |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` | |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` | |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` | |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` | |

**制御フロー命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Jmp(label)` | `br label %L` | 無条件ジャンプ |
| `JmpIf(cond, label)` | `br i1 %cond, label %L, label %fallthrough` | 条件付きジャンプ |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | 条件付き非ジャンプ |
| `Ret(Some(v))` | `ret T %v` | 戻り値あり |
| `Ret(None)` | `ret void` | 戻り値なし |

**呼び出し命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Call { dst, func, args }` | `%r = call T @func(...)` | 静的呼び出し |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call`（関数ポインタ） | 仮想メソッド呼び出し、vtable 検索経由 |
| `CallDyn { dst, func, args }` | `%r = call T %func(...)` | 動的呼び出し（クロージャ/関数ポインタ） |
| `TailCall { func, args }` | `musttail call` / `tail call` | 末尾呼び出し最適化 |

**メモリ命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Move { dst, src }` | — | 正規化後レジスタコピーとなり、SSA 構築で大部分除去可能 |
| `Load { dst, src }` | `%v = load T, T* %src` | |
| `Store { dst, src }` | `store T %src, T* %dst` | |
| `Alloc { dst, size }` | `%p = alloca T`（スタック）/ `call @malloc`（ヒープへエスケープ） | エスケープ分析が割り当て位置を決定 |
| `Free(ptr)` | `call @free(%ptr)`（ヒープ）/ —（スタック、自動回収） | |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]`（スタック）/ `call @malloc`（ヒープ） | |

**構造体/配列アクセス命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `LoadField { dst, src, field }` | `%ptr = getelementptr T, T* %src, 0, field` + `load` | |
| `StoreField { dst, field, src }` | `%ptr = getelementptr T, T* %dst, 0, field` + `store` | |
| `LoadIndex { dst, src, index }` | `%ptr = getelementptr T, T* %src, 0, %index` + `load` | |
| `StoreIndex { dst, index, src }` | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` | |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` チェーン | フィールド順に LLVM struct を構築 |

**型変換命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | ソース/ターゲット型の組合せに応じて適切な cast 命令を選択 |
| `TypeTest(val, type)` | — | コンパイル時型テスト、型タグ比較の `icmp eq` を生成 |

**所有権と借用命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Borrow { dst, src, mutable }` | — | **ゼロサイズトークン、コンパイル後完全に消滅**、IR を一切生成しない |
| `Release(val)` | — | **ゼロサイズトークン、コンパイル後完全に消滅** |
| `Move { dst, src }` | — | 所有権移動、正規化後レジスタコピーとなる |
| `Drop(val)` | `call void @T.drop(T* %val)` | 型のデストラクタを呼び出す（§7 参照） |
| `ShareRef { dst, src }` | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | コンパイラがスレッド越えか否かに応じて Arc/Rc を自動選択 |
| `ArcNew { dst, src }` | `call %T* @Arc_new(%src)` | 原子参照カウント = 1 |
| `ArcClone { dst, src }` | `call %T* @Arc_clone(%src)` | 原子的に参照カウントを増加 |
| `ArcDrop(val)` | `call void @Arc_drop(%val)` | 原子的にデクリメント + 条件付き解放 |

**並行性命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Spawn { closures, plan, result }` | スケジューラ呼び出しシーケンスに展開 | 詳細は §5、ランタイム `task_spawn` + `task_wait_all` |
| `Yield` | — | AOT パスでは spawn ブロックが同期待機、yield は不要；無視 |

**unsafe ブロックと生ポインタ命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `UnsafeBlockStart` | — | **コンパイル時マーカー、IR を生成しない** |
| `UnsafeBlockEnd` | — | **コンパイル時マーカー、IR を生成しない** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64`（またはポインタを直接コピー） | |
| `PtrDeref { dst, src }` | `%v = load T, T* %src` | |
| `PtrStore { dst, src }` | `store T %src, T* %dst` | |
| `PtrLoad { dst, src }` | `%v = load T, T* %src` | |

**文字列命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `StringLength { dst, src }` | `%len = extractvalue { i8*, i64 } %src, 1` | String は `{ ptr, len }`、長はフィールド 1 |
| `StringConcat { dst, lhs, rhs }` | `call String @yx_string_concat(%lhs, %rhs)` | ランタイムヘルパー関数 |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32` | 境界チェック含む |
| `StringFromInt { dst, src }` | `call String @yx_string_from_int(%src)` | ランタイムヘルパー関数 |
| `StringFromFloat { dst, src }` | `call String @yx_string_from_f64(%src)` | ランタイムヘルパー関数 |

**クロージャ命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `MakeClosure { dst, func: String, env }` | クロージャ構造体を割り当て + 関数ポインタ（関数名で検索）を埋める + 環境 | `{ fn_ptr, env_fields... }` |
| `LoadUpvalue { dst, upvalue_idx }` | `%v = extractvalue %env, upvalue_idx` | クロージャ環境から upvalue を読み取り |
| `StoreUpvalue { src, upvalue_idx }` | `%env = insertvalue %env, %src, upvalue_idx` | クロージャ環境に upvalue を書き込み |
| `CloseUpvalue(val)` | スタック上の upvalue をヒープにコピー | |

**その他の命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `HeapAlloc { dst, type_id }` | `call i8* @malloc(i64 size)` + 型タグ書き込み | ヒープ割り当て + 型情報 |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)` | ランタイムヘルパー関数 |

> **注意**：`Push`/`Pop`/`Dup`/`Swap` は §4.0 正規化フェーズで除去済み、翻訳表には現れない。`Borrow`/`Release` はゼロサイズのコンパイル時トークンであり、マシンコードを一切生成しない。

### 5. spawn ブロックコード生成

[RFC-024](../accepted/024-concurrency-model.md) に整合し、spawn ブロックのコンパイルは以下のステップに分かれる。

#### 5.1 セマンティクス振り返り

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // 直接子式 → タスク 1
    t2 = fetch("url2"),   // 直接子式 → タスク 2
    return (t1, t2)       // 同期待機、結果を組み立て
}
```

**ルール**（RFC-024 §2.1）：
- spawn ブロックの**直接子式**（トップレベルのカンマ区切り文）が並列タスクを作成する
- ネストした `{}` 内の式は直接子式とみなされず、独立したタスクにならない
- spawn ブロック全体は同期ブロッキングし、全タスク完了後に結果を返す

#### 5.2 コンパイルステップ

```
Step 1: 直接子式の識別
  spawn ブロック本体を走査し、トップレベル文を収集

Step 2: 依存分析
  各直接子式について、前のタスクが生成した変数を参照しているかを分析
  依存なし → 即座に並列スケジュール可能
  依存あり → 依存タスクの完了を待つようキューイング

Step 3: リソース競合検出（RFC-024 §2.5）
  同一リソース型のインスタンスが複数タスクで使われているかチェック
  同インスタンス競合 → 逐次実行順序をマーク

Step 4: タスク関数の生成
  各直接子式は独立した LLVM 関数（クロージャ）を生成

Step 5: スケジューリングコードの生成
  ランタイム scheduler の task_spawn / task_wait を呼び出す

Step 6: 結果組み立て
  全タスク出力を収集し、return タプルを組み立て
```

#### 5.3 LLVM IR 生成パターン

```llvm
; spawn ブロック入口
%task_count = 2
%tasks = alloca [2 x %TaskHandle]

; タスク 1 作成：fetch("url1")
%task1_fn = @spawn_closure_1
call @runtime_task_spawn(%tasks[0], %task1_fn, ...)

; タスク 2 作成：fetch("url2")
%task2_fn = @spawn_closure_2
call @runtime_task_spawn(%tasks[1], %task2_fn, ...)

; 全タスクを同期待機
call @runtime_task_wait_all(%tasks, %task_count)

; 戻り値を組み立て
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

コンパイラは `parse(data)` がタスク 1 が生成した `data` を参照していることを検出し、スケジューリングコード生成時に依存をマークする：

```llvm
; タスク 2 はタスク 1 への依存付きで作成
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 タスク 0（fetch）完了に依存
```

#### 5.5 リソース型の自動シリアライズ

[RFC-024 §2.5](../accepted/024-concurrency-model.md) で定義されるリソース型（`FilePath`、`HttpUrl`、`DBUrl`、`Console` およびユーザ定義リソース型）は spawn ブロック内で自動的にシリアライズされる：

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // SqliteDb（リソース型）を使用
    r2 = db.exec("INSERT ...")    // 同一インスタンス → 自動的にシリアライズ
}
```

コンパイラは同一リソースインスタンスが 2 つのタスクで使われていることを検出し、逐次依存を生成する：

```llvm
; タスク 2 はタスク 1 に依存（同リソースで自動シリアライズ）
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for データ並列

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

コンパイラは N 個の独立タスク（N = items の長さ）に展開し、最大並列数によって制限される。

### 6. FFI コード生成

> ⚠️ **依存関係の説明**：本節で定義される FFI コード生成**アーキテクチャ**（`native("x")` → `declare external @x` → マーシャリングラッパ関数 → call）は安定しており、RFC-026 構文の変更に影響されない。具体的な引数マーシャリングルール表（§6.2）および不透明型レイアウト（§6.3）は RFC-026 の定義を参照する——RFC-026 の `native()` 構文またはマーシャリングルールが変更された場合、本文書の対応するマッピング表のみを更新すればよく、アーキテクチャ層は影響を受けない。RFC-026 の現状：**レビュー中**、本文書と同じ `review/` ディレクトリにある。
>
> **承認の前提条件**：本 RFC の承認前に、RFC-026 の本文書 §6 に関連する部分（`native()` 宣言構文、引数マーシャリングルール、不透明型 `{ i8* }` レイアウト、`.drop` バインド規約）を先に凍結するか、026 と共に承認する必要がある。さもなければ §6.2/§6.3/§7 のマッピング表が実装前に陳腐化する可能性がある。

[RFC-026](./026-ffi-core-mechanism.md) に整合し、本節は FFI 呼び出しの LLVM IR 生成戦略を定義する。

#### 6.1 native() 関数宣言

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

LLVM IR にコンパイル：

```llvm
; 外部 C 関数を宣言
declare i8* @sqlite3_open(i8*)

; YaoXiang ラッパ関数（マーシャリング処理）
define { i8* } @__yx_sqlite3_open({ i8*, i64 } %filename) {
    ; マーシャリング：YaoXiang String → C 文字列
    %c_str = extractvalue { i8*, i64 } %filename, 0
    ; C 関数を呼び出す
    %raw = call i8* @sqlite3_open(i8* %c_str)
    ; アンマーシャリング：C ポインタ → 不透明型
    %result = insertvalue { i8* } undef, i8* %raw, 0
    ret { i8* } %result
}
```

**要点**：
- `native("sqlite3_open")` → `declare external @sqlite3_open`
- コンパイラが自動的にマーシャリングラッパ関数を生成
- ラッパ関数のシグネチャは YaoXiang 型を使用し、内部で C 型に変換

#### 6.2 引数マーシャリング

| 方向 | 変換 |
|------|------|
| YaoXiang `String` → C `char*` | `.ptr` フィールドを抽出して渡す |
| YaoXiang `Int32` → C `int` | 直接渡す（`i32`） |
| YaoXiang `*Void` → C `void*` | 直接渡す（`i8*`） |
| YaoXiang `T`（透過型） → C `struct T*` | アドレスを取得して渡す |
| YaoXiang `T`（不透明型） → C `struct T*` | `{ i8* }` 内のポインタを抽出して渡す |

#### 6.3 不透明型の LLVM レイアウト

[RFC-026](./026-ffi-core-mechanism.md) §4.1 で定義される不透明型：

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVM レイアウト：`{ i8* }` — C ポインタを 1 個含む構造体。

**レイアウト最適化**：不透明型に `handle: *Void` フィールドが 1 つだけある場合、直接 `i8*` を使用するよう最適化できる（外側 struct を省略）。最適化後の ABI は C ポインタと完全一致し、マーシャリングオーバーヘッドはゼロ。コンパイラはデフォルトでこの最適化を有効にし、ユーザは意識する必要がない。

#### 6.4 ?T null 許容戻り値の LLVM 表現

[RFC-026](./026-ffi-core-mechanism.md) §7.6 で定義される FFI null 許容戻り値：

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

汎用 LLVM 表現：`{ i1, { i8* } }` — 有値マーカー + データ。

**FFI null ポインタ向け最適化**：`?T` の `T` が不透明型（内部がポインタ）の場合、コンパイラは **null ポインタ = None** 最適化を使用する：

```llvm
; 最適化後の LLVM 表現：null 許容ポインタを直接使用
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None、null 以外 → Some（不透明型としてラップ）
    ret i8* %raw
}
```

呼び出し側：
```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

この最適化により `?SqliteDb` の FFI 呼び出しは**追加オーバーヘッドがゼロ**——C の null チェックと完全に等価。

#### 6.5 yx-bindgen 統合

[yx-bindgen](./026-ffi-core-mechanism.md) §6 により自動生成されるバインドファイルは、コンパイル時に通常の YaoXiang ソースコードとして処理される。コンパイラはコードが bindgen 由来であることを知る必要がない——`native()` 宣言と `unsafe {}` 型定義の処理方法は完全に同じ。

### 7. デストラクタコード生成

[RFC-009](../accepted/009-ownership-model.md) の RAII セマンティクスと [RFC-026](./026-ffi-core-mechanism.md) §7 の `.drop` 規約に整合する。

#### 7.1 .drop バインドの識別

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

コンパイラは `.drop` バインドを識別し、型メタデータにデストラクタ関数ポインタをマークする。

#### 7.2 スコープ終了時の Cleanup 挿入

```
ユーザコード：
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← スコープ終了
}

コンパイラが挿入する cleanup（逆順）：
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**挿入位置**：
- 通常のスコープ終了（`}`）
- 早期リターン（`return` の前）
- `?` エラー伝播パス（`?` の前）
- spawn ブロック終了（タスク内変数のデストラクタ）

#### 7.3 Move とデストラクタ

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有権が db2 に移転
// db は無効、ここでは db に対する drop を挿入しない
// ← スコープ終了：db2 のみ drop を挿入
```

コンパイラは Move セマンティクス（[RFC-009](../accepted/009-ownership-model.md) §1）を追跡し、変数の最終所有者に対してのみデストラクタ呼び出しを挿入する。

#### 7.4 デストラクタ失敗処理

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

### 8. コンパイル成果物構造

コンパイル成果物は以下の構成要素を含む（具体的な struct 定義は実装段階で決定）：

- **マシンコード**：LLVM がコンパイルしたオブジェクトファイル（`.o`）、すべての関数翻訳結果を含む
- **spawn メタデータ**：各 spawn ブロックのタスク関数ポインタ、依存関係、リソース競合シリアライズペア
- **FFI シンボル表**：外部 C シンボル参照（シンボル名 + 弱参照かどうか）
- **エントリポイント表**：実行可能ファイルのエントリ関数リスト
- **型情報**：リフレクションメタデータ、`.reflect` セクションに書き込み、ランタイムでオンデマンド mmap

### 9. ランタイムライブラリ

[RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md) に整合し、ランタイムは**静的ライブラリ**として最終 exe にリンクされる。

```
最終 exe 内部構造：

┌────────────────────────────────────────────┐
│  ユーザコード（ネイティブマシンコード）         │
│  ├── 通常関数（シーケンシャル実行）            │
│  ├── spawn ブロック展開（タスク関数 + スケジューラ呼び出し）│
│  ├── FFI マーシャリングラッパ関数            │
│  └── RAII デストラクタコード                 │
├────────────────────────────────────────────┤
│  ランタイム静的ライブラリ（約 500KB-1MB、プラットフォームと機能選択に依存）  │
│  ├── スレッドプール（num_workers）           │
│  ├── イベントループ（libuv / io_uring）     │
│  ├── ワークスティーリングキュー（Full Runtime のみ）│
│  ├── メモリアロケータ（jemalloc / mimalloc）│
│  └── リフレクションメタデータ（.reflect セクション、オンデマンド mmap）│
│                                              │
│  含まれないもの：                             │
│  ❌ バイトコードインタプリタ                  │
│  ❌ JIT コンパイラ                            │
│  ❌ GC                                      │
│  ❌ 仮想マシン                                │
└────────────────────────────────────────────┘
```

**重要な設計**：コンパイル時に spawn ブロックのタスク識別と依存分析を完了し、ランタイムは「タスク作成 → スレッドプールへディスパッチ → 完了待機」のみを実行する——データ構造は固定され、動作は予測可能。

> **RFC-008 とのサイズ推定の差異**：RFC-008 §4 ではスケジューラを約 200-500KB と推定しており、タスクスケジューリングコアのみを含む。本文書の 500KB-1MB 推定は、メモリアロケータ（jemalloc/mimalloc）、イベントループ（libuv/io_uring）、リフレクションメタデータセクションを追加で含む。実際のサイズはプラットフォームと機能選択に依存し、実装段階で正確な数字を示す。

**3 層ランタイムと LLVM の関係**（RFC-008 §1 に整合）：

| ランタイム | LLVM AOT 動作 |
|--------|---------------|
| **Embedded** | spawn サポートなし、シーケンシャルマシンコードを直接生成 |
| **Standard** | spawn ブロックサポート、spawn ブロック内 DAG + シングルスレッドスケジューリング（num_workers=1） |
| **Full** | spawn ブロックサポート、spawn ブロック内 DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealing サポート |

---

## 詳細設計

### モジュールディレクトリ構造

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 のディレクトリレイアウトに整合する。`[! 計画中]` マーカーは、そのファイル/ディレクトリがまだ作成されておらず、本 RFC の実装段階で導入されることを示す。

```
src/
├── frontend/                          # コンパイルフロントエンド（全バックエンド共有）
│   ├── core/
│   │   ├── spawn/                     # spawn モジュール（VM と LLVM バックエンドで共有される並行性分析）
│   │   │   ├── mod.rs                 # spawn モジュールエントリ
│   │   │   ├── placement.rs           # spawn 出現位置の正当性チェック
│   │   │   └── analysis.rs            # [! 計画中] タスク識別、依存分析、リソース競合検出
│   │   └── typecheck/
│   │       └── ...
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR 定義（VM と LLVM 共有）
│   │   └── ir_gen.rs                  # IR 生成
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs                 # オーケストレーション層（現在 BytecodeFile を出力）
│       │   ├── translator.rs          # IR → バイトコード翻訳（VM バックエンド用）
│       │   ├── emitter.rs             # バイトコードエミット + ジャンプバックフィル（VM バックエンド用）
│       │   ├── buffer.rs              # 定数プール + バイトコードバッファ（VM バックエンド用）
│       │   ├── bytecode.rs            # バイトコードフォーマット定義 + シリアライズ（VM バックエンド用）
│       │   ├── flow.rs                # レジスタ割り当て + ラベル生成 + シンボルテーブル（VM バックエンド用）
│       │   └── operand.rs             # オペランド解析（VM バックエンド用）
│       ├── lifetime/                  # ライフタイム/トークン活性分析
│       └── mono/                      # 単態化
│
├── backends/
│   ├── common/                        # 共有値/ヒープ/オペコード
│   ├── interpreter/                   # ツリーウォーキングインタプリタ（VM バックエンド）
│   ├── llvm/                          # [! 計画中] LLVM バックエンドコード生成（下記ファイル一覧参照）
│   │   ├── mod.rs                     # [! 計画中] LLVM バックエンドエントリ
│   │   ├── context.rs                 # [! 計画中] LLVM コンテキスト管理
│   │   ├── types.rs                   # [! 計画中] 型マッピング（YaoXiang → LLVM IR）
│   │   ├── values.rs                  # [! 計画中] 値マッピング
│   │   ├── func.rs                    # [! 計画中] 関数翻訳
│   │   ├── spawn.rs                   # [! 計画中] spawn ブロック展開
│   │   ├── ffi.rs                     # [! 計画中] FFI 呼び出しコード生成
│   │   └── drop.rs                    # [! 計画中] デストラクタ挿入
│   └── runtime/                       # コンパイル型ランタイム（静的ライブラリとして exe にリンク）
│       ├── engine.rs                  # タスクスケジューリングエンジン
│       ├── facade.rs                  # 外部インターフェース
│       └── task.rs                    # タスク表現
│
└── util/
    └── diagnostic/                    # エラー診断（共有）
```

> **重要な変更点**：spawn ブロック分析（タスク識別、依存分析、リソース競合検出）は `frontend/core/spawn/`（フロントエンド共有）で実装される。既存の `frontend/core/typecheck/passes/spawn_placement.rs`（spawn 出現位置チェック）は `frontend/core/spawn/placement.rs` へ移行する、詳細は RFC-024 参照。LLVM バックエンドは分析結果を消費し、対応するスケジューリングコードを生成するのみ。
>
> **現状の説明**：現在の `middle/passes/codegen/` 配下の `buffer.rs`、`emitter.rs`、`bytecode.rs`、`flow.rs`、`operand.rs` は VM バックエンドのバイトコード生成（`CodegenContext::generate()` → `BytecodeFile`）用である。LLVM バックエンドは `backends/llvm/` に実装され、interpreter バックエンドおよび runtime と同列——両者は同じ `ModuleIR` 入力を共有し、異なるターゲットフォーマット（バイトコード vs ネイティブコード）を出力する。

### プラットフォーム ABI サポート

| プラットフォーム | ターゲットトリプレット | 出力フォーマット | 呼び出し規約（FFI デフォルト） |
|------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI 呼び出しはデフォルトでプラットフォームの C 呼び出し規約を使用。ユーザは `native("symbol", cc = "stdcall")` などのオプションで上書き可能（[RFC-026](./026-ffi-core-mechanism.md) の将来の拡張に整合）。

### 浮動小数点セマンティクス一貫性（VM ↔ LLVM）

デュアルバックエンドアーキテクチャの核心的な約束は、VM（開発デバッグ）と LLVM（本番リリース）の動作が一致することである。浮動小数点演算は両実行モード間に潜在的な不整合点を持つ：

| シナリオ | リスク | 戦略 |
|------|------|------|
| NaN 伝播 | VM と LLVM は NaN の符号ビットとペイロード処理が異なる可能性 | コンパイラが IR 層で NaN 表現を正規化、NaN 比較は `fcmp uno` で統一 |
| 丸めモード | LLVM はデフォルトで round-to-nearest-even、VM はホスト CPU に依存 | 非デフォルト丸めモードを公開しない、VM と LLVM は RTNE で統一 |
| ゼロ除算 | IEEE 754 は ±Inf を定義するが、一部のプラットフォームではトラップする可能性あり | debug モードでゼロ除算をチェックし診断を報告；release モードは IEEE 754 に従う |
| `-0.0` vs `+0.0` | 比較操作で等価にならない可能性あり | IEEE 754 ルールで統一：`+0.0 == -0.0` |
| 非正規化数 | 一部のプラットフォームは flush-to-zero | LLVM は `denormal-fp-math` 属性を有効にせず、完全な IEEE 754 セマンティクスを保持 |

> **テスト戦略**：バックエンド横断の浮動小数点一貫性テストスイートを実装する——同じ YaoXiang ソースコードを VM と LLVM バックエンドでそれぞれ実行し、出力を値ごとに比較する。このテスト群は CI の強制ゲート。

---

## トレードオフ

### 利点

1. **パフォーマンス**：AOT コンパイルは解釈実行より 10-100 倍高速
2. **統一フロントエンド**：VM と LLVM は同一フロントエンドを共有、動作は完全に一致
3. **ゼロスケジューリングオーバーヘッド**：通常コードはシーケンシャルマシンコードを直接生成、spawn ブロック外に DAG オーバーヘッドなし
4. **静的リンク**：外部ランタイム依存なし、単一 exe でデプロイ可能
5. **ゼロ GC**：RAII による決定論的デストラクタ、ポーズなし
6. **FFI ゼロオーバーヘッド**：`?T` null ポインタ最適化、不透明型レイアウト最適化により、FFI 呼び出しコストは C と同等
7. **コンパイル時分析**：spawn ブロックのタスク識別と依存分析はコンパイル時に完了、ランタイムは実行のみ

### 欠点

1. **LLVM 統合の複雑さ**：inkwell API と LLVM IR を深く理解する必要
2. **コンパイル時間**：AOT コンパイルはインタプリタより遅い（一度だけのコスト）
3. **デバッグ体験**：ネイティブコードデバッグには DWARF/PDB シンボルサポートが必要（コンパイラがデバッグ情報を生成する必要）
4. **インクリメンタルコンパイル**：大規模プロジェクトのインクリメンタルコンパイルには追加設計が必要
5. **浮動小数点セマンティクス一貫性**：VM と LLVM は NaN 伝播、丸めモード、ゼロ除算などの境界動作で差異が生じる可能性があり、正規化戦略によりデュアルバックエンドの動作一貫性を保証する必要がある（§10 参照）

### 関連 RFC との一貫性

| RFC | 一貫性 |
|-----|--------|
| RFC-024 spawn ブロック並行性モデル | ✅ spawn ブロック直接子式 → タスクディスパッチ |
| RFC-008 ランタイムアーキテクチャ | ✅ デュアルバックエンド + スケジューラ静的ライブラリ + モジュールディレクトリ構造 |
| RFC-009 所有権モデル v9 | ✅ `&T`/`&mut T` トークン（ゼロサイズ）、`ref T`（ファットポインタ）、`?T`（Option） |
| RFC-026 FFI コアメカニズム | ✅ `native()` → declare + マーシャリング、`.drop` → RAII cleanup |

---

## 代替案

| 案 | 説明 | 選択しない理由 |
|------|------|------|
| インタプリタのみ使用 | AOT 不要 | パフォーマンス不足 |
| 純粋静的コンパイル（ランタイムなし） | スケジューラをリンクしない | spawn ブロックはランタイムタスクスケジューリングが必要 |
| Cranelift バックエンド | より高速なコンパイル | ランタイム性能は LLVM に及ばず、将来のオプションとして検討 |
| 外部 LLVM runtime をリンク | LLVM 内蔵ランタイムを使用 | 不要な依存関係の導入 |

---

## 実装戦略

### フェーズ分割

#### フェーズ 1：基本フレームワーク
- [ ] inkwell 依存を追加
- [ ] LLVM コンテキスト初期化を実装（`context.rs`）
- [ ] 基本型マッピングを実装（`types.rs`）

#### フェーズ 2：関数翻訳
- [ ] 関数宣言翻訳を実装（`func.rs`）
- [ ] 基本命令翻訳を実装（算術、制御フロー、呼び出し）（`translator.rs`）
- [ ] 値マッピングを実装（`values.rs`）

#### フェーズ 3：所有権型翻訳
- [ ] `&T`/`&mut T` トークン（ゼロサイズ、コンパイル後消滅）を実装
- [ ] `ref T`（ファットポインタ `{ i64*, T* }`）を実装
- [ ] `?T`（`{ i1, T }` tagged union）を実装
- [ ] `List(T)`（`{ T*, i64, i64 }`）を実装
- [ ] Move セマンティクス追跡を実装（デストラクタ挿入判定用）

#### フェーズ 4：spawn ブロックコード生成
- [ ] `spawn_placement.rs` の分析結果を消費
- [ ] 直接子式 → タスク関数生成
- [ ] 依存タスクスケジューリングコード生成
- [ ] リソース競合シリアライズ
- [ ] spawn for 展開

#### フェーズ 5：FFI コード生成
- [ ] `native()` → `declare external`（`ffi.rs`）
- [ ] 引数マーシャリング / 戻り値アンマーシャリング
- [ ] 不透明型レイアウト（単一フィールド最適化含む）
- [ ] `?T` null ポインタ最適化（FFI 専用）

#### フェーズ 6：デストラクタコード生成
- [ ] `.drop` バインド識別
- [ ] スコープ終了 cleanup 挿入（逆順）（`drop.rs`）
- [ ] 早期リターンパス cleanup
- [ ] `?` エラー伝播パス cleanup

#### フェーズ 7：ランタイムライブラリリンク
- [ ] `runtime_task_spawn` / `runtime_task_wait_all` などのランタイム関数を実装
- [ ] ランタイム静的ライブラリをリンク
- [ ] エンドツーエンド統合テスト

### 依存関係

- RFC-024（spawn ブロック並行性）→ フェーズ 4 の入力
- RFC-009 v9（所有権）→ フェーズ 3、6 の入力
- RFC-008（ランタイムアーキテクチャ）→ フェーズ 7 の入力
- RFC-026（FFI メカニズム）→ フェーズ 5 の入力

---

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| コア | サブタスクの遅延作成、オンデマンド作成 |
| 参考価値 | spawn ブロック内のタスクオンデマンドスケジューリングの理論的基礎 |

**核心思想**：タスクを即座に作成するのではなく、遅延作成する。親タスクがサブタスクの値を必要とした時点でサブタスクを作成する。これにより細粒度並列タスクのパフォーマンスオーバーヘッド問題を解決する[^1]。YaoXiang の spawn ブロックスケジューリングは、この思想を借用している——タスクはコンパイル時に識別されるが、ランタイムでスレッドプールにオンデマンドでディスパッチされる。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| コア | ランタイム適応型スケジューリング、追加状態なし |
| 参考価値 | Full Runtime WorkStealing スケジューラ設計の参考 |

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| コア | 単一代入言語、Dataflow グラフ、暗黙的並列性 |
| 参考価値 | Dataflow モデルの産業レベル応用における実現可能性の証明 |

**重要な違い**：SISAL の並列性は**暗黙的**——言語は単一代入セマンティクスであり、コンパイラが自動的に全プログラムのデータ依存グラフを分析して並列性を決定する。YaoXiang の並列性は**明示的**——ユーザは `spawn {}` ブロックで並列領域をマークし、コンパイラは spawn ブロック内でのみ依存を分析する。これにより SISAL の全プログラム分析の複雑さを回避しつつ、ユーザが並列動作を制御できる。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| コア | Future 構築、Lazy Task Creation 実装 |
| 参考価値 | 具体的な実装の参考 |

### 比較まとめ

| 技術 | 遅延作成 | 並列性マーカー | 分析範囲 | 所有権 |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | 暗黙的 | 全プログラム | N/A |
| Lazy Scheduling[^2] | ✅ | 暗黙的 | 全プログラム | N/A |
| SISAL[^3] | ✅ | 暗黙的（単一代入） | 全プログラム | N/A |
| Mul-T[^4] | ✅ | 明示的（future） | 呼び出し点 | N/A |
| **YaoXiang** | ✅ | **明示的（spawn ブロック）** | **spawn ブロック内** | **✅（Move + トークン + ref）** |

**YaoXiang の革新**：並列性マーカーを「各関数呼び出し」（future）から「構造化ブロック」（spawn）へと昇格させ、ユーザは通常コードを書き、必要に応じて spawn ブロックを置く。分析範囲は spawn ブロック内に限定され、コンパイルが効率的で動作が制御可能。

---

## 付録

### 付録 A：Rust async との比較

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル成果物 | ステートマシン + マシンコード | マシンコード + spawn タスクメタデータ |
| ランタイム | tokio | 静的リンクスケジューラ（約 500KB-1MB） |
| 並行性マーカー | async/await キーワード | `spawn { }` ブロック |
| タスク作成 | コンパイル時にステートマシン生成 | コンパイル時に直接子式を識別 → タスク関数 |
| カラー関数 | async 感染 | **関数カラーリングなし** |
| 同期待機 | `.await` | spawn ブロックが自動的に同期ブロッキング |
| メモリ管理 | GC（ランタイム） | **RAII（決定論的）** |
| 共有メカニズム | `Arc::new()` + 手動 Weak | **`ref` キーワード（コンパイラが Rc/Arc を自動選択）** |

### 付録 B：設計決定記録

| 決定 | 決定内容 | 日付 |
|------|------|------|
| LLVM AOT 採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| 並行性モデル整合 | RFC-024 spawn ブロック直接子式モデルに整合 | 2026-06-10 |
| DAG 分析範囲 | spawn ブロック内、spawn ブロック跨ぎなし（RFC-024 に整合） | 2026-06-05 |
| 所有権モデル整合 | RFC-009 v9 に整合：`&T`/`&mut T` トークン + `ref` キーワード | 2026-06-10 |
| デュアルバックエンドモデル | VM（開発）+ LLVM（本番）、RFC-008 に整合 | 2026-05-11 |
| スケジューラ形態 | 静的ライブラリとして exe にリンク、約 500KB-1MB（プラットフォームと機能選択に依存）、GC なし | 2026-05-11 |
| FFI コード生成 | RFC-026 統合：`native()` declare + マーシャリング | 2026-06-10 |
| デストラクタ | `.drop` → RAII cleanup 挿入、RFC-026 §7 に整合 | 2026-06-10 |
| 副作用処理 | `@IO`/`@Pure` 推論を削除、RFC-024 リソース型に置換 | 2026-06-10 |
| リフレクションメタデータ | exe の .reflect セクションにコンパイル、mmap オンデマンドロード | 2026-05-11 |
| 論文引用 | Lazy Task Creation などを保持、YaoXiang との差異を明確化 | 2026-02-16 |

---

## 参考文献

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT.

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland.

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory.

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024: spawn ブロックベースの並行性モデル](../accepted/024-concurrency-model.md)
- [RFC-008: Runtime 並行性モデルとスケジューラ疎結合設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有権モデル設計](../accepted/009-ownership-model.md)
- [RFC-026: FFI コアメカニズム](./026-ffi-core-mechanism.md)

---

## ライフサイクルと帰趣

| 状態 | 場所 | 説明 |
|------|------|------|
| **ドラフト** | `docs/design/rfc/` | 著者ドラフト、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/review/` | コミュニティディスカッションとフィードバックを公開 |
| **承認済み** | `docs/design/rfc/accepted/` | 公式設計ドキュメントとなる |
| **却下** | `docs/design/rfc/` | RFC ディレクトリに保持 |

> 現状：**承認済み** — RFC-024 spawn ブロック並行性モデル、RFC-009 v9 所有権モデル、RFC-026 FFI メカニズムに整合済み
```