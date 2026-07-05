---
title: "RFC-018：LLVM AOT コンパイラ設計"
status: "承認済み"
author: "晨煦"
created: "2026-02-15"
updated: "2026-07-05（GitHub Issue #14、#134 同期；実装状況分析を追加）"
issue: "#14"
tracking_issue: "https://github.com/ChenXu233/YaoXiang/issues/134"
---

# RFC-018：LLVM AOT コンパイラ設計

> **参考文献**:
> - [RFC-024：spawn ブロックベースの並行性モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 並行性モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
> - [RFC-026：FFI コアメカニズム](./026-ffi-core-mechanism.md)
> - [RFC-010：統一型構文](../accepted/010-unified-type-syntax.md)

> **廃止**:
> - 旧版「ボトムアップ自動 DAG 分析」モデル — RFC-024 spawn ブロック直接子式モデルに置き換え
> - `@IO`/`@Pure` 暗黙的副作用推論 — RFC-024 リソース型メカニズムに置き換え
> - `Arc(T)` 型マッピング — RFC-009 v9 `ref` キーワードに置き換え

## 概要

本文書は YaoXiang 言語の LLVM AOT（事前コンパイル）コンパイラを設計する。LLVM バックエンドは VM バックエンド（インタプリタ）と同一のコンパイルフロントエンドを共有し、[RFC-008](../accepted/008-runtime-concurrency-model.md) で定義されたデュアルバックエンドアーキテクチャを構成する：VM は開発・デバッグ用、LLVM は本番リリース用。

**中核的責務**：

```
ソースコード → フロントエンド（共有）→ IR → LLVM Codegen → .o → スケジューラ静的ライブラリをリンク → exe
```

コンパイラは YaoXiang ソースコードをネイティブマシンコードにコンパイルする：

| 言語機能 | コンパイル戦略 |
|----------|----------------|
| 通常コード | シーケンシャルなマシンコード、ゼロスケジューリングオーバーヘッド |
| `spawn { }` ブロック | 直接の子式 → タスクディスパッチ + 同期待機（[RFC-024](../accepted/024-concurrency-model.md) 整合） |
| `native("symbol")` | LLVM `declare external` + 引数マーシャリング（[RFC-026](./026-ffi-core-mechanism.md) 整合） |
| `.drop` デストラクタ | RAII cleanup コード挿入（[RFC-009](../accepted/009-ownership-model.md) 整合） |
| `&T` / `&mut T` トークン | ゼロサイズ型、コンパイル後に消滅 |
| `ref T` 共有 | `{ refcount_ptr, data_ptr }` ファットポインタ、コンパイラが Rc/Arc を自動選択 |

**RFC-024 との関係**：RFC-024 は spawn ブロックの**ユーザーセマンティクス**（直接子式によるタスク作成、同期ブロッキング待機）を定義する。本文書はこれらのセマンティクスが**マシンコードへどうコンパイルされるか**を定義する。

**RFC-026 との関係**：RFC-026 は FFI の**ユーザー構文**（`native()`、`[0]` メソッドバインド、`.drop`）を定義する。本文書は FFI 呼び出しが**LLVM IR をどう生成するか**を定義する。

---

## 動機

### なぜ LLVM AOT コンパイラが必要か？

現在 YaoXiang は実行バックエンドとしてインタプリタのみを持つ：

| 問題 | 影響 |
|------|------|
| パフォーマンスボトルネック | インタプリタ実行はマシンコードより 10-100 倍遅い |
| デプロイの複雑さ | インタプリタとランタイムの同梱が必要 |
| 本番環境 | インタプリタはパフォーマンスに敏感なシナリオに適さない |

### デュアルバックエンドモデルにおける LLVM

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 はデュアルバックエンドアーキテクチャを定義する：

```
                    ┌─────────────────────┐
                    │   コンパイルフロントエンド（統一）   │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn 分析       │
                    │   → エスケープ分析     │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM バックエンド（開発）│     │  LLVM バックエンド（本番）│
      │   IR → インタプリタ実行│     │  IR → ネイティブコード │
      │   ステップデバッグ     │     │  スケジューラ静的ライブラリをリンク│
      │   高速イテレーション   │     │  .exe を出力      │
      └───────────────────┘     └───────────────────┘
```

両バックエンドの**動作は完全に一致**する — 違いは実行方法のみ。同一のソースコード、同一の型検査、同一の spawn 分析結果。

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
      ├── spawn 展開：直接の子式 → タスク関数 + スケジューラ呼び出し
      ├── FFI 展開：native() 呼び出し → declare + マーシャリング
      └── デストラクタ挿入：スコープ終了 → .drop() 呼び出し
  → LLVM 最適化 + ターゲットコード生成
  → ランタイム静的ライブラリをリンク → 実行可能ファイル
```

### 2. コンパイルフロー

```
フェーズ 1: フロントエンド（VM バックエンドと共有）
  - 解析、型検査、spawn ブロック分析、エスケープ分析
  - 出力：型注釈付き IR

フェーズ 2: LLVM IR 生成
  - 型マッピング、関数宣言、命令翻訳
  - 出力：LLVM Module

フェーズ 3: LLVM 最適化
  - 標準 LLVM 最適化パイプライン（O0/O1/O2/O3）
  - インライン化、定数畳み込み、デッドコード除去

フェーズ 4: ターゲットコード生成
  - LLVM TargetMachine → .o ファイル
  - プラットフォーム：Linux (ELF)、macOS (Mach-O)、Windows (COFF)

フェーズ 5: リンク
  - ランタイム静的ライブラリ（スケジューラ、アロケータ）をリンク
  - 出力：実行可能ファイル
```

### 3. 型マッピング

#### 3.1 YaoXiang → LLVM IR 型マッピング

| YaoXiang 型 | LLVM IR 型 | 説明 |
|-------------|-----------|------|
| `Int` | `i64` | デフォルト 64 ビット符号付き整数 |
| `Int32` | `i32` | 明示的 32 ビット整数（主に FFI 用） |
| `Float` | `f64` | デフォルト 64 ビット浮動小数点 |
| `Float32` | `f32` | 明示的 32 ビット浮動小数点（主に FFI 用） |
| `Bool` | `i1` | ブール値 |
| `Char` | `i32` | Unicode コードポイント |
| `String` | `{ i8*, i64 }` | ポインタ + バイト長 |
| `Void` | `{}` | ゼロサイズ空型 |
| `&T` | — | ゼロサイズトークン、コンパイル後に消滅し、IR を一切生成しない |
| `&mut T` | — | ゼロサイズトークン、コンパイル後に消滅し、IR を一切生成しない |
| `ref T` | `{ i64*, T* }` | ファットポインタ（参照カウントポインタ + データポインタ） |
| `*T` | `T*` | 生ポインタ（raw pointer） |
| `[T; N]` | `[N x T]` | 固定長配列 |
| `List(T)` | `{ T*, i64, i64 }` | データポインタ + 長さ + 容量 |
| 構造体 | 対応する LLVM struct | フィールドは定義順にレイアウト |
| レコード enum | `{ i64, [max_payload_size] }` | タグ + 最大ペイロードの union |
| `?T` | `{ i1, T }` | 値存在フラグ + データ（汎用表現） |
| FFI 不透明型 | `{ i8* }` | C ポインタのラッパー |
| 関数ポインタ | `T (...)*` | 関数ポインタ型 |

> **`&T` / `&mut T` のゼロランタイムオーバーヘッド**：[RFC-009](../accepted/009-ownership-model.md) §2.7 は、コンパイラ内部でトークンにブランド識別子（コンパイル時一意整数）を割り当てることを定義しており、モノモーフィゼーションとインライン化の後、ブランドは完全に消滅する — 生成されたマシンコードにはトークンの痕跡は一切存在しない。

#### 3.2 FFI 引数型マッピング

[RFC-026](./026-ffi-core-mechanism.md) §2.2 と整合し、LLVM IR 列を補足する：

| C 型 | YaoXiang 型 | LLVM IR | 説明 |
|------|-------------|---------|------|
| `int` | `Int32` | `i32` | |
| `long` | `Int64` | `i64` | |
| `float` | `Float32` | `f32` | |
| `double` | `Float64` | `f64` | |
| `char` | `Char` | `i32` | C char → YaoXiang Char（Unicode 互換） |
| `char*` | `String` | `{ i8*, i64 }` | マーシャリング：C string → YaoXiang String |
| `bool` | `Bool` | `i1` | |
| `size_t` | `Uint` | `i64` | |
| `void*` | `*Void` | `i8*` | |
| `struct T*` | `T`（透過型） | `T*` | ポインタ渡し |
| `typedef struct T T` | `T`（不透明型） | `{ i8* }` | C ポインタをラップ |

### 4. IR 正規化と命令翻訳

#### 4.0 IR 正規化（スタック → レジスタ）

現在の IR（`src/middle/core/ir.rs`）はスタック操作命令（`Push`/`Pop`/`Dup`/`Swap`）を含むが、これらはバイトコード VM 用に設計されている。LLVM IR は SSA 形式であり、スタック操作を受け付けない。

**処理戦略**：LLVM パスは命令翻訳の前に、まず軽量な正規化 pass を通過する：

| スタック命令 | 正規化戦略 |
|--------------|-----------|
| `Push(r)` | `stack.push(r)` を記録、IR を生成しない |
| `Pop(r)` | `r = stack.pop()`、`load` を生成（スタックスロットから） |
| `Dup` | `stack.push(stack.top())`、IR を生成しない |
| `Swap` | スタック先頭の 2 要素を交換、IR を生成しない |

正規化後、すべてのオペランドはレジスタ/ローカル変数参照となり、スタック操作は完全に除去される。この pass は `translator.rs` の最初のステップとして実行される。

> **なぜ IR レベルでスタック命令を除去しないのか？** VM バックエンドはスタックセマンティクスを必要とする。LLVM 翻訳入口で正規化することで、両バックエンドの IR 共有が維持される — 各バックエンドは同一の IR をそれぞれのニーズに応じて消費する。
>
> **前提**：IR 生成フェーズはスタックバランスを保証する — すべての制御フローパスが同一プログラム点に到達したとき、スタック深さが一致する（VM バイトコードバックエンドは同一の前提に依存し、そうでなければバイトコード実行がエラーになる）。正規化 pass はこの前提をチェックしない；違反した場合、LLVM バックエンドは未定義動作を生成する。

#### 4.1 命令翻訳表

以下、`Instruction` enum の各バリアントに対する LLVM IR 翻訳戦略を列挙する。命令名は `src/middle/core/ir.rs` と完全に一致する。

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
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | 条件付きでジャンプしない |
| `Ret(Some(v))` | `ret T %v` | 戻り値あり |
| `Ret(None)` | `ret void` | 戻り値なし |

**呼び出し命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Call { dst, func, args }` | `%r = call T @func(...)` | 静的呼び出し |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call`（関数ポインタ） | 仮想メソッド呼び出し、vtable 経由でルックアップ |
| `CallDyn { dst, func, args }` | `%r = call T %func(...)` | 動的呼び出し（クロージャ/関数ポインタ） |
| `TailCall { func, args }` | `musttail call` / `tail call` | 末尾呼び出し最適化 |

**メモリ命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Move { dst, src }` | — | 正規化後レジスタコピーとなり、SSA 構築で大部分が消去可能 |
| `Load { dst, src }` | `%v = load T, T* %src` | |
| `Store { dst, src }` | `store T %src, T* %dst` | |
| `Alloc { dst, size }` | `%p = alloca T`（スタック）/ `call @malloc`（エスケープしてヒープ） | エスケープ分析が配置を決定 |
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
| `TypeTest(val, type)` | — | コンパイル時型テスト、`icmp eq` で型タグを比較 |

**所有権と借用命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Borrow { dst, src, mutable }` | — | **ゼロサイズトークン、コンパイル後に完全に消滅し、IR を一切生成しない** |
| `Release(val)` | — | **ゼロサイズトークン、コンパイル後に完全に消滅** |
| `Move { dst, src }` | — | 所有権の移転、正規化後レジスタコピーとなる |
| `Drop(val)` | `call void @T.drop(T* %val)` | 型のデストラクタを呼び出す（§7 参照） |
| `ShareRef { dst, src }` | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | コンパイラがスレッド越えかどうかに応じて Arc/Rc を自動選択 |
| `ArcNew { dst, src }` | `call %T* @Arc_new(%src)` | アトミック参照カウント = 1 |
| `ArcClone { dst, src }` | `call %T* @Arc_clone(%src)` | アトミックに参照カウントをインクリメント |
| `ArcDrop(val)` | `call void @Arc_drop(%val)` | アトミックにデクリメント + 条件付き解放 |

**並行命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `Spawn { closures, plan, result }` | スケジューラ呼び出しシーケンスに展開 | 詳細は §5、ランタイム `task_spawn` + `task_wait_all` |
| `Yield` | — | AOT パスでは spawn ブロックが同期待機するため yield は不要；無視される |

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
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32` | 境界チェックを含む |
| `StringFromInt { dst, src }` | `call String @yx_string_from_int(%src)` | ランタイムヘルパー関数 |
| `StringFromFloat { dst, src }` | `call String @yx_string_from_f64(%src)` | ランタイムヘルパー関数 |

**クロージャ命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `MakeClosure { dst, func: String, env }` | クロージャ構造体を割り当て、関数ポインタ（関数名でルックアップ）と環境を填充 | `{ fn_ptr, env_fields... }` |
| `LoadUpvalue { dst, upvalue_idx }` | `%v = extractvalue %env, upvalue_idx` | クロージャ環境から upvalue を読み取る |
| `StoreUpvalue { src, upvalue_idx }` | `%env = insertvalue %env, %src, upvalue_idx` | クロージャ環境に書き込む |
| `CloseUpvalue(val)` | スタック上の upvalue をヒープにコピー | |

**その他の命令**：

| IR 命令 | LLVM IR | 説明 |
|---------|---------|------|
| `HeapAlloc { dst, type_id }` | `call i8* @malloc(i64 size)` + 型タグ書き込み | ヒープ割り当て + 型情報 |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)` | ランタイムヘルパー関数 |

> **注意**：`Push`/`Pop`/`Dup`/`Swap` は §4.0 正規化フェーズですでに除去されており、翻訳表には現れない。`Borrow`/`Release` はゼロサイズのコンパイル時トークンであり、マシンコードを一切生成しない。

### 5. spawn ブロックコード生成

[RFC-024](../accepted/024-concurrency-model.md) と整合し、spawn ブロックのコンパイルは以下のステップに分けられる。

#### 5.1 セマンティクスの復習

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // 直接の子式 → タスク 1
    t2 = fetch("url2"),   // 直接の子式 → タスク 2
    return (t1, t2)       // 同期待機、結果を組み立て
}
```

**ルール**（RFC-024 §2.1）：
- spawn ブロックの**直接の子式**（トップレベル・コンマ区切りの文）は並列タスクを作成する
- ネストされた `{}` 内の式は直接の子式とはみなされず、独立したタスクにはならない
- spawn ブロック全体は同期的にブロックし、全タスクの完了を待ってから返す

#### 5.2 コンパイルステップ

```
ステップ 1: 直接の子式の識別
  spawn ブロックの本体を走査し、トップレベルの文を収集

ステップ 2: 依存分析
  各直接の子式について、先行タスクが生成した変数を参照しているかを分析
  依存なし → 即座に並列スケジュール可能
  依存あり → 依存タスクの完了を待つようキューイング

ステップ 3: リソース競合検出（RFC-024 §2.5）
  同一リソース型のインスタンスが複数のタスクで使用されていないかを確認
  同一インスタンスで競合 → 逐次実行順序にマーク

ステップ 4: タスク関数の生成
  各直接の子式を独立した LLVM 関数（クロージャ）として生成

ステップ 5: スケジューリングコードの生成
  ランタイムスケジューラの task_spawn / task_wait を呼び出す

ステップ 6: 結果の組み立て
  全タスクの出力を収集し、return タプルを組み立てる
```

#### 5.3 LLVM IR 生成パターン

```llvm
; spawn ブロックの入口
%task_count = 2
%tasks = alloca [2 x %TaskHandle]

; タスク 1 の作成：fetch("url1")
%task1_fn = @spawn_closure_1
call @runtime_task_spawn(%tasks[0], %task1_fn, ...)

; タスク 2 の作成：fetch("url2")
%task2_fn = @spawn_closure_2
call @runtime_task_spawn(%tasks[1], %task2_fn, ...)

; 全タスクを同期待機
call @runtime_task_wait_all(%tasks, %task_count)

; 戻り値を組み立てる
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
;                                                 タスク 0（fetch）の完了に依存
```

#### 5.5 リソース型の自動シリアライズ

[RFC-024 §2.5](../accepted/024-concurrency-model.md) で定義されたリソース型（`FilePath`、`HttpUrl`、`DBUrl`、`Console` およびユーザ定義のリソース型）は spawn ブロック内で自動的にシリアライズされる：

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // SqliteDb（リソース型）を使用
    r2 = db.exec("INSERT ...")    // 同一インスタンス → 自動シリアライズ
}
```

コンパイラは同一リソースインスタンスが 2 つのタスクで使用されていることを検出し、逐次依存を生成する：

```llvm
; タスク 2 はタスク 1 に依存（同一リソースによる自動シリアライズ）
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for データ並列

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

コンパイラはこれを N 個の独立したタスク（N = items の長さ）に展開し、最大並列数の制限を受ける。

### 6. FFI コード生成

> ⚠️ **依存関係の説明**：本節で定義する FFI コード生成の**アーキテクチャ**（`native("x")` → `declare external @x` → マーシャリングラッパー関数 → call）は安定しており、RFC-026 構文の変更によって変動しない。具体的な引数マーシャリングルール表（§6.2）および不透明型レイアウト（§6.3）は RFC-026 の定義を引用する — RFC-026 の `native()` 構文またはマーシャリングルールが変更された場合は、本文書の対応する対応表を更新するだけで、アーキテクチャ層に影響しない。RFC-026 の現状：**レビュー中**、本 RFC と同じ `review/` ディレクトリにある。
>
> **承認の前提条件**：本 RFC が承認される前に、RFC-026 の本文書 §6 に関連する部分（`native()` 宣言構文、引数マーシャリングルール、不透明型 `{ i8* }` レイアウト、`.drop` バインド規約）を先に凍結するか、026 と同時に承認するべきである。そうでなければ §6.2/§6.3/§7 の対応表が実装前に時代遅れになる可能性がある。

[RFC-026](./026-ffi-core-mechanism.md) と整合し、本節は FFI 呼び出しの LLVM IR 生成戦略を定義する。

#### 6.1 native() 関数宣言

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

LLVM IR へコンパイル：

```llvm
; 外部 C 関数を宣言
declare i8* @sqlite3_open(i8*)

; YaoXiang ラッパー関数（マーシャリングを処理）
define { i8* } @__yx_sqlite3_open({ i8*, i64 } %filename) {
    ; マーシャリング：YaoXiang String → C string
    %c_str = extractvalue { i8*, i64 } %filename, 0
    ; C 関数を呼び出し
    %raw = call i8* @sqlite3_open(i8* %c_str)
    ; アンマーシャリング：C ポインタ → 不透明型
    %result = insertvalue { i8* } undef, i8* %raw, 0
    ret { i8* } %result
}
```

**要点**：
- `native("sqlite3_open")` → `declare external @sqlite3_open`
- コンパイラがマーシャリングラッパー関数を自動生成
- ラッパー関数のシグネチャは YaoXiang 型を使用し、内部で C 型へ変換する

#### 6.2 引数マーシャリング

| 方向 | 変換 |
|------|------|
| YaoXiang `String` → C `char*` | `.ptr` フィールドを抽出して渡す |
| YaoXiang `Int32` → C `int` | そのまま渡す（`i32`） |
| YaoXiang `*Void` → C `void*` | そのまま渡す（`i8*`） |
| YaoXiang `T`（透過型） → C `struct T*` | アドレスを取って渡す |
| YaoXiang `T`（不透明型） → C `struct T*` | `{ i8* }` からポインタを抽出して渡す |

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

LLVM レイアウト：`{ i8* }` — C ポインタを 1 つ含む構造体。

**レイアウト最適化**：不透明型に `handle: *Void` フィールドが 1 つしかない場合、直接 `i8*` を使用するように最適化できる（外側 struct を省略）。最適化後の ABI は C ポインタと完全一致し、マーシャリングオーバーヘッドはゼロ。コンパイラはデフォルトでこの最適化を有効化し、ユーザはこれを意識しない。

#### 6.4 ?T null 許容戻り値の LLVM 表現

[RFC-026](./026-ffi-core-mechanism.md) §7.6 で定義された FFI の null 許容戻り値：

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

汎用 LLVM 表現：`{ i1, { i8* } }` — 値存在フラグ + データ。

**FFI null ポインタに対する最適化**：`?T` の `T` が不透明型（内部がポインタ）の場合、コンパイラは **null ポインタ = None** 最適化を使用する：

```llvm
; 最適化後の LLVM 表現：null 許容ポインタを直接使用
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

この最適化により、`?SqliteDb` の FFI 呼び出しは**追加オーバーヘッドがゼロ** — C の null チェックと完全に等価である。

#### 6.5 yx-bindgen 統合

[yx-bindgen](./026-ffi-core-mechanism.md) §6 により自動生成されたバインドファイルは、コンパイル時に通常の YaoXiang ソースコードとして処理される。コンパイラはコードが bindgen 由来であることを知る必要がない — `native()` 宣言と `unsafe {}` 型定義の処理方法は完全に同一である。

### 7. デストラクタコード生成

[RFC-009](../accepted/009-ownership-model.md) の RAII セマンティクスおよび [RFC-026](./026-ffi-core-mechanism.md) §7 の `.drop` 規約と整合する。

#### 7.1 .drop バインドの識別

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

コンパイラは `.drop` バインドを識別し、型メタデータにデストラクタ関数ポインタをマークする。

#### 7.2 スコープ終了時の cleanup 挿入

```
ユーザのコード：
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
- spawn ブロック終了（タスク内変数のデストラクト）

#### 7.3 Move とデストラクト

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有権を db2 に移転
// db は無効、ここでは db に対する drop を挿入しない
// ← スコープ終了：db2 に対してのみ drop を挿入
```

コンパイラは Move セマンティクス（[RFC-009](../accepted/009-ownership-model.md) §1）を追跡し、変数の最終的な保持者に対してのみデストラクタ呼び出しを挿入する。

#### 7.4 デストラクト失敗の処理

```llvm
; debug モード：デストラクタの戻り値をチェック
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

### 8. コンパイル出力の構造

コンパイル出力は以下の構成要素を含む（具体的な struct 定義は実装段階で決定）：

- **マシンコード**：LLVM がコンパイルしたオブジェクトファイル（`.o`）、すべての関数翻訳結果を含む
- **spawn メタデータ**：各 spawn ブロックのタスク関数ポインタ、依存関係、リソース競合のシリアライズペア
- **FFI シンボル表**：外部 C シンボル参照（シンボル名 + 弱参照かどうか）
- **エントリポイント表**：実行可能ファイルのエントリ関数リスト
- **型情報**：リフレクションメタデータ、`.reflect` セクションに書き込み、ランタイムが必要に応じて mmap

### 9. ランタイムライブラリ

[RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md) と整合し、ランタイムは**静的ライブラリ**形式で最終 exe にリンクされる。

```
最終 exe の内部構造：

┌────────────────────────────────────────────┐
│  ユーザコード（ネイティブマシンコード）          │
│  ├── 通常関数（シーケンシャル実行）            │
│  ├── spawn ブロック展開（タスク関数 + スケジューラ呼び出し）│
│  ├── FFI マーシャリングラッパー関数           │
│  └── RAII デストラクタコード                 │
├────────────────────────────────────────────┤
│  ランタイム静的ライブラリ（約 500KB-1MB、プラットフォームと機能選択に依存）│
│  ├── スレッドプール（num_workers）           │
│  ├── イベントループ（libuv / io_uring）      │
│  ├── ワークスティーリングキュー（Full Runtime のみ）│
│  ├── メモリアロケータ（jemalloc / mimalloc） │
│  └── リフレクションメタデータ（.reflect セクション、mmap でオンデマンド）│
│                                              │
│  含まないもの：                                │
│  ❌ バイトコードインタプリタ                  │
│  ❌ JIT コンパイラ                            │
│  ❌ GC                                       │
│  ❌ 仮想マシン                                 │
└────────────────────────────────────────────┘
```

**重要な設計**：spawn ブロックのタスク識別と依存分析はコンパイル時に完了し、ランタイムは「タスク作成 → スレッドプールへのディスパッチ → 完了待機」を実行するだけ — データ構造は固定で、動作は予測可能。

> **RFC-008 のサイズ推定との差異**：RFC-008 §4 はスケジューラを約 200-500KB と推定しており、タスクスケジューリングコアのみを含む。本文書の 500KB-1MB 推定は、メモリアロケータ（jemalloc/mimalloc）、イベントループ（libuv/io_uring）、リフレクションメタデータセクションを別途含む。実際のサイズはプラットフォームと機能選択に依存し、実装段階で正確な数値を示す。

**3 層ランタイムと LLVM の関係**（RFC-008 §1 と整合）：

| ランタイム | LLVM AOT 動作 |
|--------|---------------|
| **Embedded** | spawn サポートなし、シーケンシャルマシンコードを直接生成 |
| **Standard** | spawn ブロックサポート、spawn ブロック内 DAG + シングルスレッドスケジューリング（num_workers=1） |
| **Full** | spawn ブロックサポート、spawn ブロック内 DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealing サポート |

---

## 詳細設計

### モジュールディレクトリ構造

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 のディレクトリレイアウトと整合する。`[! 計画中]` マーカーは、そのファイル/ディレクトリがまだ作成されておらず、本 RFC の実装段階で導入されることを示す。

```
src/
├── frontend/                          # コンパイルフロントエンド（すべてのバックエンドで共有）
│   ├── core/
│   │   ├── spawn/                     # spawn モジュール（VM と LLVM バックエンドで共有される並行性分析）
│   │   │   ├── mod.rs                 # spawn モジュールエントリ
│   │   │   ├── placement.rs           # spawn 出現位置の合法性チェック
│   │   │   └── analysis.rs            # [! 計画中] タスク識別、依存分析、リソース競合検出
│   │   └── typecheck/
│   │       └── ...
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR 定義（VM と LLVM で共有）
│   │   └── ir_gen.rs                  # IR 生成
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs                 # オーケストレーション層（現在 BytecodeFile を出力）
│       │   ├── translator.rs          # IR → バイトコード翻訳（VM バックエンド用）
│       │   ├── emitter.rs             # バイトコード発行 + ジャンプバックパッチ（VM バックエンド用）
│       │   ├── buffer.rs              # 定数プール + バイトコードバッファ（VM バックエンド用）
│       │   ├── bytecode.rs            # バイトコードフォーマット定義 + シリアライズ（VM バックエンド用）
│       │   ├── flow.rs                # レジスタ割り当て + ラベル生成 + シンボルテーブル（VM バックエンド用）
│       │   └── operand.rs             # オペランド解析（VM バックエンド用）
│       ├── lifetime/                  # ライフタイム/トークン活性分析
│       └── mono/                      # モノモーフィゼーション
│
├── backends/
│   ├── common/                        # 共有値/ヒープ/オペコード
│   ├── interpreter/                   # ツリー traversal インタプリタ（VM バックエンド）
│   ├── llvm/                          # [! 計画中] LLVM バックエンドコード生成（以下のファイル一覧参照）
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
    └── diagnostic/                    # 診断エラー（共有）
```

> **主要な変更点**：spawn ブロック分析（タスク識別、依存分析、リソース競合検出）は `frontend/core/spawn/`（フロントエンド共有）に実装される。既存の `frontend/core/typecheck/passes/spawn_placement.rs`（spawn 出現位置チェック）は `frontend/core/spawn/placement.rs` に移行する。詳細は RFC-024 を参照。LLVM バックエンドは分析結果のみを消費し、対応するスケジューリングコードを生成する。
>
> **現状の説明**：現在 `middle/passes/codegen/` 配下の `buffer.rs`、`emitter.rs`、`bytecode.rs`、`flow.rs`、`operand.rs` は VM バックエンドのバイトコード生成（`CodegenContext::generate()` → `BytecodeFile`）のために存在する。LLVM バックエンドは `backends/llvm/` に実装され、interpreter バックエンドおよび runtime と同列となる — 両者は同一の `ModuleIR` 入力を共有し、異なるターゲットフォーマット（バイトコード vs ネイティブコード）を出力する。

### プラットフォーム ABI サポート

| プラットフォーム | ターゲット triple | 出力フォーマット | 呼び出し規約（FFI デフォルト） |
|----------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI 呼び出しはデフォルトでプラットフォームの C 呼び出し規約を使用する。ユーザは `native("symbol", cc = "stdcall")` などのオプションで上書きできる（[RFC-026](./026-ffi-core-mechanism.md) の将来の拡張と整合）。

### 浮動小数点セマンティクスの一貫性（VM ↔ LLVM）

デュアルバックエンドアーキテクチャの核心的な約束は、VM（開発デバッグ）と LLVM（本番リリース）の動作が一致することである。浮動小数点演算は両実行モードで潜在的な不整合点を持つ：

| シナリオ | リスク | 戦略 |
|------|------|------|
| NaN 伝播 | VM と LLVM は NaN の符号ビットとペイロードの扱いが異なる可能性がある | コンパイラは IR レベルで NaN 表現を正規化し、NaN 比較は `fcmp uno` を統一使用 |
| 丸めモード | LLVM のデフォルトは round-to-nearest-even、VM はホスト CPU に依存 | 非デフォルトの丸めモードを公開せず、VM と LLVM は RTNE を統一使用 |
| ゼロ除算 | IEEE 754 は ±Inf を定義するが、一部のプラットフォームではトラップする可能性 | debug モードでゼロ除算をチェックして診断を報告；release モードは IEEE 754 に従う |
| `-0.0` vs `+0.0` | 比較操作が等価にならない可能性がある | IEEE 754 ルールを統一使用：`+0.0 == -0.0` |
| 非正規化数 | 一部のプラットフォームでは flush-to-zero | LLVM は `denormal-fp-math` 属性を有効にせず、完全な IEEE 754 セマンティクスを保持 |

> **テスト戦略**：クロスバックエンドの浮動小数点一貫性テストスイートを実装する — 同一の YaoXiang ソースコードを VM と LLVM バックエンドでそれぞれ実行し、出力を値ごとに比較する。このテストスイートは CI の必須ゲートである。

---

## トレードオフ

### 利点

1. **パフォーマンス**：AOT コンパイルはインタプリタ実行より 10-100 倍高速
2. **統一フロントエンド**：VM と LLVM が同一のフロントエンドを共有し、動作が完全に一致
3. **ゼロスケジューリングオーバーヘッド**：通常コードはシーケンシャルマシンコードを直接生成、spawn ブロック外に DAG オーバーヘッドなし
4. **静的リンク**：外部ランタイム依存がなく、単一の exe でデプロイ可能
5. **ゼロ GC**：RAII による決定論的デストラクト、ポーズなし
6. **FFI ゼロオーバーヘッド**：`?T` null ポインタ最適化、不透明型レイアウト最適化により、FFI 呼び出しコストは C と等価
7. **コンパイル時分析**：spawn ブロックのタスク識別と依存分析はコンパイル時に完了し、ランタイムは実行のみ

### 欠点

1. **LLVM 統合の複雑さ**：inkwell API と LLVM IR の深い理解が必要
2. **コンパイル時間**：AOT コンパイルはインタプリタより遅い（一回限りのコスト）
3. **デバッグ体験**：ネイティブコードのデバッグには DWARF/PDB シンボルサポートが必要（コンパイラはデバッグ情報を生成する必要がある）
4. **インクリメンタルコンパイル**：大規模プロジェクトのインクリメンタルコンパイルには追加設計が必要
5. **浮動小数点セマンティクスの一貫性**：VM と LLVM は NaN 伝播、丸めモード、ゼロ除算などの境界動作で差異がある可能性があり、正規化戦略によりデュアルバックエンドの動作一致を保証する必要がある（§10 参照）

### 関連 RFC との一貫性

| RFC | 一貫性 |
|-----|--------|
| RFC-024 spawn ブロック並行性モデル | ✅ spawn ブロック直接の子式 → タスクディスパッチ |
| RFC-008 ランタイムアーキテクチャ | ✅ デュアルバックエンド + スケジューラ静的ライブラリ + モジュールディレクトリ構造 |
| RFC-009 所有権モデル v9 | ✅ `&T`/`&mut T` トークン（ゼロサイズ）、`ref T`（ファットポインタ）、`?T`（Option） |
| RFC-026 FFI コアメカニズム | ✅ `native()` → declare + マーシャリング、`.drop` → RAII cleanup |

---

## 代替案

| 案 | 説明 | 採用しない理由 |
|------|------|------|
| インタプリタのみ | AOT 不要 | パフォーマンス不足 |
| 純粋な静的コンパイル（ランタイムなし） | スケジューラをリンクしない | spawn ブロックにはランタイムタスクスケジューリングが必要 |
| Cranelift バックエンド | より高速なコンパイル速度 | ランタイムパフォーマンスが LLVM に劣る、将来的にオプションのバックエンドとして |
| 外部 LLVM ランタイムのリンク | LLVM 内蔵ランタイムを使用 | 不要な依存関係を導入 |

---

## 実装戦略

### フェーズ区分

#### フェーズ 1：基本フレームワーク
- [ ] inkwell 依存関係を追加
- [ ] LLVM コンテキスト初期化を実装（`context.rs`）
- [ ] 基本型マッピングを実装（`types.rs`）

#### フェーズ 2：関数翻訳
- [ ] 関数宣言翻訳を実装（`func.rs`）
- [ ] 基本命令翻訳を実装（算術、制御フロー、呼び出し）（`translator.rs`）
- [ ] 値マッピングを実装（`values.rs`）

#### フェーズ 3：所有権型の翻訳
- [ ] `&T`/`&mut T` トークンを実装（ゼロサイズ、コンパイル後に消滅）
- [ ] `ref T` を実装（ファットポインタ `{ i64*, T* }`）
- [ ] `?T` を実装（`{ i1, T }` tagged union）
- [ ] `List(T)` を実装（`{ T*, i64, i64 }`）
- [ ] Move セマンティクス追跡を実装（デストラクタ挿入判定用）

#### フェーズ 4：spawn ブロックコード生成
- [ ] `spawn_placement.rs` の分析結果を消費
- [ ] 直接の子式 → タスク関数生成
- [ ] 依存タスクのスケジューリングコード生成
- [ ] リソース競合のシリアライズ
- [ ] spawn for の展開

#### フェーズ 5：FFI コード生成
- [ ] `native()` → `declare external`（`ffi.rs`）
- [ ] 引数マーシャリング / 戻り値アンマーシャリング
- [ ] 不透明型レイアウト（単一フィールド最適化を含む）
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

- RFC-024（spawn ブロック並行性）→ フェーズ 4 への入力
- RFC-009 v9（所有権）→ フェーズ 3、6 への入力
- RFC-008（ランタイムアーキテクチャ）→ フェーズ 7 への入力
- RFC-026（FFI メカニズム）→ フェーズ 5 への入力

---

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| 中核 | 子タスクを遅延作成、オンデマンドで作成 |
| 参考価値 | spawn ブロック内のタスクオンデマンドスケジューリングの理論的基盤 |

**中核思想**：タスクを即座に作成せず、遅延作成する。親タスクが子タスクの値を必要としたときに初めて子タスクを作成する。これは細粒度並列タスクのパフォーマンスオーバーヘッド問題を解決する[^1]。YaoXiang の spawn ブロックスケジューリングはこの思想を借用している — タスクはコンパイル時に識別されるが、ランタイムではスレッドプールへオンデマンドでディスパッチされる。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| 中核 | ランタイム適応的スケジューリング、追加状態なし |
| 参考価値 | Full Runtime WorkStealing スケジューラ設計の参考 |

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| 中核 | 単一代入言語、Dataflow グラフ、暗黙的並列 |
| 参考価値 | 産業レベルアプリケーションにおける Dataflow モデルの実現可能性の証明 |

**重要な違い**：SISAL の並列性は**暗黙的**である — 言語は単一代入セマンティクスで、コンパイラが全プログラムのデータ依存グラフを自動分析して並列性を決定する。YaoXiang の並列性は**明示的**である — ユーザが `spawn {}` ブロックで並列領域をマークし、コンパイラは spawn ブロック内でのみ依存を分析する。これは SISAL の全プログラム分析の複雑さを回避しつつ、ユーザの並列動作の制御を保持する。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 中核 | Future 構築、Lazy Task Creation の実装 |
| 参考価値 | 具体的な実装の参考 |

### 比較サマリ

| 技術 | 遅延作成 | 並列マーク | 分析範囲 | 所有権 |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | 暗黙的 | 全プログラム | N/A |
| Lazy Scheduling[^2] | ✅ | 暗黙的 | 全プログラム | N/A |
| SISAL[^3] | ✅ | 暗黙的（単一代入） | 全プログラム | N/A |
| Mul-T[^4] | ✅ | 明示的（future） | 呼び出し点 | N/A |
| **YaoXiang** | ✅ | **明示的（spawn ブロック）** | **spawn ブロック内** | **✅（Move + トークン + ref）** |

**YaoXiang のイノベーション**：並列マークを「各関数呼び出し」（future）から「構造化ブロック」（spawn）へと昇格させ、ユーザは通常通りコードを書き、並列が必要な箇所に spawn ブロックを置く。分析範囲は spawn ブロック内に限定され、コンパイル効率が高く、動作が制御可能。

---

## 付録

### 付録 A：Rust async との比較

| 機能 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル出力 | ステートマシン + マシンコード | マシンコード + spawn タスクメタデータ |
| ランタイム | tokio | 静的リンクスケジューラ（約 500KB-1MB） |
| 並行性マーク | async/await キーワード | `spawn { }` ブロック |
| タスク作成 | コンパイル時にステートマシンを生成 | コンパイル時に直接の子式を識別 → タスク関数 |
| カラー関数 | async 感染 | **関数カラーリングなし** |
| 同期待機 | `.await` | spawn ブロックが自動同期ブロッキング |
| メモリ管理 | GC（ランタイム） | **RAII（決定論的）** |
| 共有メカニズム | `Arc::new()` + 手動 Weak | **`ref` キーワード（コンパイラが Rc/Arc を自動選択）** |

### 付録 B：設計決定記録

| 決定 | 結論 | 日付 |
|------|------|------|
| LLVM AOT 採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| 並行性モデル整合 | RFC-024 spawn ブロック直接子式モデルと整合 | 2026-06-10 |
| DAG 分析範囲 | spawn ブロック内、spawn ブロックを跨がない（RFC-024 と整合） | 2026-06-05 |
| 所有権モデル整合 | RFC-009 v9 と整合：`&T`/`&mut T` トークン + `ref` キーワード | 2026-06-10 |
| デュアルバックエンドモデル | VM（開発）+ LLVM（本番）、RFC-008 と整合 | 2026-05-11 |
| スケジューラ形態 | 静的ライブラリとして exe にリンク、約 500KB-1MB（プラットフォームと機能に依存）、GC なし | 2026-05-11 |
| FFI コード生成 | RFC-026 を統合：`native()` declare + マーシャリング | 2026-06-10 |
| デストラクタ | `.drop` → RAII cleanup 挿入、RFC-026 §7 と整合 | 2026-06-10 |
| 副作用処理 | `@IO`/`@Pure` 推論を削除、RFC-024 リソース型に変更 | 2026-06-10 |
| リフレクションメタデータ | exe の .reflect セクションにコンパイル、mmap でオンデマンドロード | 2026-05-11 |
| 論文引用 | Lazy Task Creation などを保持、YaoXiang の違いを明確化 | 2026-02-16 |

---

## 参考文献

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT.

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland.

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory.

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024：spawn ブロックベースの並行性モデル](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 並行性モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
- [RFC-026：FFI コアメカニズム](./026-ffi-core-mechanism.md)

---

## ライフサイクルと帰属

| ステータス | 場所 | 説明 |
|------|------|------|
| **ドラフト** | `docs/design/rfc/` | 著者の草稿、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/review/` | コミュニティの議論とフィードバックを公開 |
| **承認済み** | `docs/design/rfc/accepted/` | 公式設計ドキュメントとなる |
| **却下** | `docs/design/rfc/` | RFC ディレクトリに保持 |

> 現在のステータス：**承認済み** — RFC-024 spawn ブロック並行性モデル、RFC-009 v9 所有権モデル、RFC-026 FFI メカニズムと整合済み