```markdown
---
title: "RFC-018：LLVM AOT コンパイラ設計"
status: "レビュー中"
author: "晨煦"
created: "2026-02-15"
updated: "2026-06-10（RFC-024 spawn ブロック並行モデル、RFC-009 v9 所有権モデル、RFC-026 FFI 機構に整合）"
---

# RFC-018：LLVM AOT コンパイラ設計

> **参考**:
> - [RFC-024：spawn ブロックベースの並行モデル](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 並行モデルとスケジューラ疎結合設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
> - [RFC-026：FFI コア機構](./026-ffi-core-mechanism.md)
> - [RFC-010：統一型構文](../accepted/010-unified-type-syntax.md)

> **廃止**:
> - 旧版「ボトムアップ自動 DAG 解析」モデル — RFC-024 spawn ブロック直接子式モデルに置き換え
> - `@IO`/`@Pure` 暗黙副作用推論 — RFC-024 リソース型機構に置き換え
> - `Arc(T)` 型マッピング — RFC-009 v9 `ref` キーワードに置き換え

## 概要

本文書は YaoXiang 言語の LLVM AOT（Ahead-of-Time）コンパイラを設計する。LLVM バックエンドと VM バックエンド（インタプリタ）は同一のコンパイルフロントエンドを共有し、[RFC-008](../accepted/008-runtime-concurrency-model.md) で定義されるデュアルバックエンドアーキテクチャを構成する：VM は開発・デバッグ用、LLVM は本番リリース用。

**中核的な責務**：

```
ソースコード → フロントエンド（共有）→ IR → LLVM Codegen → .o → スケジューラ静的ライブラリをリンク → exe
```

コンパイラは YaoXiang ソースコードをネイティブマシンコードにコンパイルする。各言語機能に対して：

| 言語機能 | コンパイル戦略 |
|----------|----------------|
| 通常コード | 逐次マシンコード、ゼロスケジューリングオーバーヘッド |
| `spawn { }` ブロック | 直接子式 → タスクディスパッチ + 同期待機（[RFC-024](../accepted/024-concurrency-model.md) に整合） |
| `native("symbol")` | LLVM `declare external` + 引数マーシャリング（[RFC-026](./026-ffi-core-mechanism.md) に整合） |
| `.drop` デストラクタ | RAII クリーンアップコード挿入（[RFC-009](../accepted/009-ownership-model.md) に整合） |
| `&T` / `&mut T` トークン | ゼロサイズ型、コンパイル後消失 |
| `ref T` 共有 | `{ refcount_ptr, data_ptr }` ファットポインタ、コンパイラが自動的に Rc/Arc を選択 |

**RFC-024 との関係**：RFC-024 は spawn ブロックの**ユーザセマンティクス**（直接子式によるタスク作成、同期ブロッキング待機）を定義する。本文書はこれらのセマンティクスが**マシンコードへコンパイルされる方法**を定義する。

**RFC-026 との関係**：RFC-026 は FFI の**ユーザ構文**（`native()`、`[0]` メソッドバインディング、`.drop`）を定義する。本文書は FFI 呼び出しが**LLVM IR を生成する方法**を定義する。

---

## 動機

### なぜ LLVM AOT コンパイラが必要なのか？

現在 YaoXiang はインタプリタのみを実行バックエンドとしている：

| 問題 | 影響 |
|------|------|
| 性能ボトルネック | インタプリタ実行はマシンコードより 10-100 倍遅い |
| デプロイの複雑さ | インタプリタとランタイムの同梱が必要 |
| 本番環境 | インタプリタは性能重視のシナリオに適さない |

### デュアルバックエンドモデルにおける LLVM

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 はデュアルバックエンドアーキテクチャを定義する：

```
                    ┌─────────────────────┐
                    │   コンパイルフロントエンド（統一） │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn 解析       │
                    │   → エスケープ解析    │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM バックエンド（開発）│     │  LLVM バックエンド（本番）│
      │   IR → インタプリタ実行│     │  IR → ネイティブコード │
      │   ステップデバッグ   │     │  スケジューラ静的ライブラリをリンク │
      │   高速イテレーション │     │  .exe を出力       │
      └───────────────────┘     └───────────────────┘
```

両バックエンドの**振る舞いは完全に一致**する — 違いは実行方式のみ。同一のソースコード、同一の型検査、同一の spawn 解析結果。

---

## 提案

### 1. コンパイラアーキテクチャ

LLVM バックエンドはコンパイルパイプラインの最終段階に位置し、フロントエンドから IR を受け取り、ネイティブコードを生成する：

```
ソースコード
  → Lexer / Parser（frontend/core/）
  → TypeCheck + spawn 解析（frontend/core/typecheck/）
  → IR 生成（middle/core/ir_gen.rs）
  → LLVM Codegen（middle/passes/codegen/llvm/）
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
  - 解析、型検査、spawn ブロック解析、エスケープ解析
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
| `Int32` | `i32` | 明示的な 32 ビット整数（主に FFI 用） |
| `Float` | `f64` | デフォルト 64 ビット浮動小数点 |
| `Float32` | `f32` | 明示的な 32 ビット浮動小数点（主に FFI 用） |
| `Bool` | `i1` | ブール値 |
| `Char` | `i32` | Unicode コードポイント |
| `String` | `{ i8*, i64 }` | ポインタ + バイト長 |
| `Void` | `{}` | ゼロサイズ空型 |
| `&T` | — | ゼロサイズトークン、コンパイル後消失、IR を一切生成しない |
| `&mut T` | — | ゼロサイズトークン、コンパイル後消失、IR を一切生成しない |
| `ref T` | `{ i32*, T* }` | ファットポインタ（参照カウントポインタ + データポインタ） |
| `*T` | `T*` | 生ポインタ |
| `[T; N]` | `[N x T]` | 固定長配列 |
| `List(T)` | `{ T*, i64, i64 }` | データポインタ + 長さ + 容量 |
| 構造体 | 対応する LLVM struct | フィールドは定義順にレイアウト |
| 判別共用体（タグ付き enum） | `{ i64, [max_payload_size] }` | タグ + 最大 payload の union |
| `?T` | `{ i1, T }` | 有値タグ + データ（汎用表現） |
| FFI 不透明型 | `{ i8* }` | C ポインタのラッパ |
| 関数ポインタ | `T (...)*` | 関数ポインタ型 |

> **`&T` / `&mut T` ゼロランタイムオーバーヘッド**：[RFC-009](../accepted/009-ownership-model.md) §2.7 は、コンパイラが内部でトークンにブランド識別子（コンパイル時一意整数）を割り当て、モノモーフィゼーションとインライン化後にブランドが完全に消失することを定義する — 生成されるマシンコードにはトークンの痕跡は一切存在しない。

#### 3.2 FFI 引数型マッピング

[RFC-026](./026-ffi-core-mechanism.md) §2.2 に整合し、LLVM IR 列を補足する：

| C 型 | YaoXiang 型 | LLVM IR | 説明 |
|--------|---------------|---------|------|
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
| `typedef struct T T` | `T`（不透明型） | `{ i8* }` | C ポインタのラッパ |

### 4. 命令翻訳

各 IR 命令は対応する LLVM IR 命令に直接マッピングされる。簡略マッピング表：

| IR 命令 | LLVM IR |
|---------|---------|
| `BinaryOp { add }` | `add` |
| `BinaryOp { sub }` | `sub` |
| `BinaryOp { mul }` | `mul` |
| `BinaryOp { div }` | `sdiv` / `fdiv` |
| `Compare { eq }` | `icmp eq` / `fcmp oeq` |
| `CallStatic` | `call` |
| `CallIndirect` | `call`（関数ポインタ経由） |
| `Load` | `load` |
| `Store` | `store` |
| `LoadElement` | `getelementptr` + `load` |
| `Alloca` | `alloca` |
| `Branch` | `br` |
| `BranchCond` | `br i1` |
| `Return` | `ret` |

詳細な命令翻訳は `middle/passes/codegen/llvm/` の具体的実装を参照。

### 5. spawn ブロックコード生成

[RFC-024](../accepted/024-concurrency-model.md) に整合し、spawn ブロックのコンパイルは以下のステップに分けられる。

#### 5.1 セマンティクスのおさらい

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // 直接子式 → タスク 1
    t2 = fetch("url2"),   // 直接子式 → タスク 2
    return (t1, t2)       // 同期待機、結果を組み立て
}
```

**ルール**（RFC-024 §2.1）：
- spawn ブロックの**直接子式**（トップレベルのカンマ区切り文）は並列タスクを作成する
- ネストされた `{}` 内の式は直接子式とみなされず、独立したタスクにはならない
- spawn ブロック全体が同期ブロッキングし、全タスク完了後に return する

#### 5.2 コンパイルステップ

```
Step 1: 直接子式の識別
  spawn ブロック本体を走査し、トップレベル文を収集

Step 2: 依存解析
  各直接子式について、前のタスクが生成した変数を参照しているかを解析
  依存なし → 即時並列スケジューリング可能
  依存あり → 依存タスク完了を待つキューに挿入

Step 3: リソース競合検出（RFC-024 §2.5）
  同一リソース型のインスタンスが複数タスクで使用されていないか確認
  同一インスタンス競合 → 逐次実行順序をマーク

Step 4: タスク関数の生成
  各直接子式に対して独立した LLVM 関数（クロージャ）を生成

Step 5: スケジューリングコードの生成
  ランタイム scheduler の task_spawn / task_wait を呼び出す

Step 6: 結果組み立て
  全タスクの出力を収集し、return タプルを組み立て
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

#### 5.5 リソース型の自動直列化

[RFC-024 §2.5](../accepted/024-concurrency-model.md) で定義されるリソース型（`FilePath`、`HttpUrl`、`DBUrl`、`Console` およびユーザ定義リソース型）は spawn ブロック内で自動的に直列化される：

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // SqliteDb（リソース型）を使用
    r2 = db.exec("INSERT ...")    // 同一インスタンス → 自動直列化
}
```

コンパイラは同一リソースインスタンスが 2 つのタスクで使用されていることを検出し、直列依存を生成する：

```llvm
; タスク 2 はタスク 1 に依存（同リソース自動直列化）
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for データ並列

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

コンパイラは N 個の独立タスク（N = items の長さ）に展開し、最大並列数の制限を受ける。

### 6. FFI コード生成

[RFC-026](./026-ffi-core-mechanism.md) に整合し、本節は FFI 呼び出しの LLVM IR 生成戦略を定義する。

#### 6.1 native() 関数宣言

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

LLVM IR へのコンパイル結果：

```llvm
; 外部 C 関数を宣言
declare i8* @sqlite3_open(i8*)

; YaoXiang ラッパ関数（マーシャリング処理）
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
- コンパイラが自動的にマーシャリングラッパ関数を生成
- ラッパ関数のシグネチャは YaoXiang 型を使用し、内部で C 型に変換

#### 6.2 引数マーシャリング

| 方向 | 変換 |
|------|------|
| YaoXiang `String` → C `char*` | `.ptr` フィールドを抽出して渡す |
| YaoXiang `Int32` → C `int` | 直接渡す（`i32`） |
| YaoXiang `*Void` → C `void*` | 直接渡す（`i8*`） |
| YaoXiang `T`（透過型） → C `struct T*` | アドレスを取って渡す |
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

LLVM レイアウト：`{ i8* }` — C ポインタを 1 つ含む構造体。

**レイアウト最適化**：不透明型が `handle: *Void` フィールド 1 つだけの場合、直接 `i8*` を使用するよう最適化可能（外側 struct を省略）。最適化後の ABI は C ポインタと完全一致し、マーシャリングオーバーヘッドはゼロ。コンパイラはデフォルトでこの最適化を有効化し、ユーザは意識する必要がない。

#### 6.4 ?T null 許容戻り値の LLVM 表現

[RFC-026](./026-ffi-core-mechanism.md) §7.6 で定義される FFI null 許容戻り値：

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

汎用 LLVM 表現：`{ i1, { i8* } }` — 有値タグ + データ。

**FFI null ポインタ向け最適化**：`?T` の `T` が不透明型（内部がポインタ）の場合、コンパイラは **null ポインタ = None** 最適化を使用：

```llvm
; 最適化後の LLVM 表現：null 許容ポインタを直接使用
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None、non-null → Some（不透明型にラップ）
    ret i8* %raw
}
```

呼び出し側：
```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

この最適化により `?SqliteDb` の FFI 呼び出しは**追加オーバーヘッドゼロ** — C の null チェックと完全等価。

#### 6.5 yx-bindgen 統合

[yx-bindgen](./026-ffi-core-mechanism.md) §6 が自動生成するバインディングファイルは、コンパイル時に通常の YaoXiang ソースコードとして処理される。コンパイラはコードが bindgen 由来であることを認識する必要がない — `native()` 宣言と `unsafe {}` 型定義の処理方法は完全に同一。

### 7. デストラクタコード生成

[RFC-009](../accepted/009-ownership-model.md) の RAII セマンティクスと [RFC-026](./026-ffi-core-mechanism.md) §7 の `.drop` 規約に整合する。

#### 7.1 .drop バインディング識別

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

コンパイラは `.drop` バインディングを識別し、型メタデータにデストラクタ関数ポインタをマークする。

#### 7.2 スコープ終了時のクリーンアップ挿入

```
ユーザコード：
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← スコープ終了
}

コンパイラが挿入するクリーンアップ（逆順）：
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**挿入位置**：
- 通常のスコープ終了（`}`）
- 早期 return（`return` の前）
- `?` エラー伝播パス（`?` の前）
- spawn ブロック終了（タスク内変数のデストラクタ）

#### 7.3 Move とデストラクタ

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有権を db2 に移転
// db は無効、ここでは db に対する drop は挿入されない
// ← スコープ終了：db2 に対してのみ drop を挿入
```

コンパイラは Move セマンティクス（[RFC-009](../accepted/009-ownership-model.md) §1）を追跡し、変数の最終保持者の位置にのみデストラクタ呼び出しを挿入する。

#### 7.4 デストラクタ失敗処理

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

### 8. コンパイル成果物構造

```rust
/// コンパイル成果物：マシンコード + メタデータ
pub struct CompiledArtifact {
    /// LLVM コンパイル済みマシンコード（オブジェクトファイル）
    machine_code: Vec<u8>,

    /// spawn ブロックメタデータ：タスク関数ポインタ + 依存関係
    spawn_metadata: SpawnMetadata,

    /// FFI シンボルテーブル：外部シンボル参照
    ffi_symbols: Vec<FfiSymbol>,

    /// エントリポイント表
    entries: Vec<EntryPoint>,

    /// 型情報（リフレクションメタデータ、.reflect セクションに書き込まれ、必要に応じて mmap）
    type_info: TypeInfo,
}

/// spawn ブロックメタデータ
pub struct SpawnMetadata {
    /// 各 spawn ブロックの記述
    blocks: Vec<SpawnBlockInfo>,
}

pub struct SpawnBlockInfo {
    /// spawn ブロック内各直接子式に対応するタスク関数
    tasks: Vec<TaskInfo>,
    /// リソース競合：直列化が必要なタスクペア
    serialize_pairs: Vec<(usize, usize)>,
}

pub struct TaskInfo {
    /// タスク関数の LLVM 関数ポインタ
    pub func_ptr: usize,
    /// 依存するタスクインデックス（空 = 依存なし、即時実行可能）
    pub deps: Vec<usize>,
}

/// FFI シンボル参照
pub struct FfiSymbol {
    /// C シンボル名
    pub symbol_name: String,
    /// 弱参照か否か（欠落を許容）
    pub weak: bool,
}
```

### 9. ランタイムライブラリ

[RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md) に整合し、ランタイムは**静的ライブラリ**形式で最終 exe にリンクされる。

```
最終 exe の内部構造：

┌────────────────────────────────────────────┐
│  ユーザコード（ネイティブマシンコード）          │
│  ├── 通常関数（逐次実行）                    │
│  ├── spawn ブロック展開（タスク関数 + スケジューリング呼び出し） │
│  ├── FFI マーシャリングラッパ関数              │
│  └── RAII デストラクタコード                  │
├────────────────────────────────────────────┤
│  ランタイム静的ライブラリ（~200-500KB）         │
│  ├── スレッドプール（num_workers）            │
│  ├── イベントループ（libuv / io_uring）      │
│  ├── ワークスティーリングキュー（Full Runtime のみ）│
│  ├── メモリアロケータ（jemalloc / mimalloc）│
│  └── リフレクションメタデータ（.reflect セクション、必要に応じて mmap）│
│                                              │
│  含まないもの：                                │
│  ❌ バイトコードインタプリタ                    │
│  ❌ JIT コンパイラ                            │
│  ❌ GC                                       │
│  ❌ 仮想マシン                                  │
└────────────────────────────────────────────┘
```

**重要な設計**：spawn ブロックのタスク識別と依存解析はコンパイル時に完了し、ランタイムは「タスク作成 → スレッドプールへのディスパッチ → 完了待機」のみを実行する — データ構造は固定で、振る舞いは予測可能。

**3 層ランタイムと LLVM の関係**（RFC-008 §1 に整合）：

| ランタイム | LLVM AOT の振る舞い |
|-----------|---------------------|
| **Embedded** | spawn サポートなし、直接逐次マシンコードを生成 |
| **Standard** | spawn ブロックサポート、spawn ブロック内 DAG + シングルスレッドスケジューリング（num_workers=1） |
| **Full** | spawn ブロックサポート、spawn ブロック内 DAG + マルチスレッドスケジューリング（num_workers>1）、WorkStealing サポート |

---

## 詳細設計

### モジュールディレクトリ構造

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 のディレクトリレイアウトに整合：

```
src/
├── frontend/                          # コンパイルフロントエンド（全バックエンド共有）
│   └── core/typecheck/
│       └── spawn_placement.rs         # spawn ブロック解析（タスク識別、依存解析、リソース競合検出）
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR 定義（VM と LLVM 共有）
│   │   └── ir_gen.rs                  # IR 生成
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs
│       │   ├── translator.rs          # IR → LLVM IR メイン翻訳
│       │   └── llvm/
│       │       ├── mod.rs             # LLVM バックエンドエントリ
│       │       ├── context.rs         # LLVM コンテキスト管理
│       │       ├── types.rs           # 型マッピング（YaoXiang → LLVM IR）
│       │       ├── values.rs          # 値マッピング
│       │       ├── func.rs            # 関数翻訳
│       │       ├── spawn.rs           # spawn ブロック展開
│       │       ├── ffi.rs             # FFI 呼び出しコード生成
│       │       └── drop.rs            # デストラクタ挿入
│       ├── lifetime/                  # ライフタイム / トークン活性解析
│       └── mono/                      # モノモーフィゼーション
│
├── backends/
│   ├── common/                        # 共有値 / ヒープ / オペコード
│   ├── interpreter/                   # ツリーウォーキングインタプリタ（VM バックエンド）
│   └── runtime/                       # コンパイル型ランタイム（静的ライブラリとして exe にリンク）
│       ├── engine.rs                  # タスクスケジューリングエンジン
│       ├── facade.rs                  # 外部インターフェース
│       └── task.rs                    # タスク表現
│
└── util/
    └── diagnostic/                    # エラー診断（共有）
```

> **重要な変更点**：spawn ブロック解析は `frontend/core/typecheck/spawn_placement.rs`（フロントエンド共有）にあり、LLVM バックエンドにはない。LLVM バックエンドは解析結果を消費して対応するスケジューリングコードを生成するのみ。

### プラットフォーム ABI サポート

| プラットフォーム | ターゲットトリプル | 出力フォーマット | 呼び出し規約（FFI デフォルト） |
|------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI 呼び出しはデフォルトでプラットフォームの C 呼び出し規約を使用。ユーザは `native("symbol", cc = "stdcall")` 等のオプションで上書き可能（[RFC-026](./026-ffi-core-mechanism.md) の将来の拡張に整合）。

---

## トレードオフ

### 利点

1. **性能**：AOT コンパイルはインタプリタ実行より 10-100 倍速い
2. **統一フロントエンド**：VM と LLVM が同一フロントエンドを共有し、振る舞いが完全一致
3. **ゼロスケジューリングオーバーヘッド**：通常コードは直接逐次マシンコードを生成、spawn ブロック外に DAG オーバーヘッドなし
4. **静的リンク**：外部ランタイム依存なし、単一 exe でデプロイ可能
5. **ゼロ GC**：RAII による決定論的デストラクタ、ポーズなし
6. **FFI ゼロオーバーヘッド**：`?T` null ポインタ最適化、不透明型レイアウト最適化により、FFI 呼び出しコストは C と等価
7. **コンパイル時解析**：spawn ブロックのタスク識別と依存解析はコンパイル時に完了、ランタイムは実行のみ

### 欠点

1. **LLVM 統合の複雑さ**：inkwell API と LLVM IR の深い理解が必要
2. **コンパイル時間**：AOT コンパイルはインタプリタより遅い（一回限りのコスト）
3. **デバッグ体験**：ネイティブコードのデバッグには DWARF/PDB シンボルサポートが必要（コンパイラがデバッグ情報を生成する必要がある）
4. **インクリメンタルコンパイル**：大規模プロジェクトのインクリメンタルコンパイルには追加設計が必要

### 関連 RFC との一貫性

| RFC | 一貫性 |
|-----|--------|
| RFC-024 spawn ブロック並行モデル | ✅ spawn ブロック直接子式 → タスクディスパッチ |
| RFC-008 ランタイムアーキテクチャ | ✅ デュアルバックエンド + スケジューラ静的ライブラリ + モジュールディレクトリ構造 |
| RFC-009 所有権モデル v9 | ✅ `&T`/`&mut T` トークン（ゼロサイズ）、`ref T`（ファットポインタ）、`?T`（Option） |
| RFC-026 FFI コア機構 | ✅ `native()` → declare + マーシャリング、`.drop` → RAII クリーンアップ |

---

## 代替案

| 案 | 説明 | 採用しない理由 |
|------|------|-----------|
| インタプリタのみ | AOT 不要 | 性能不足 |
| 純粋静的コンパイル（ランタイムなし） | スケジューラをリンクしない | spawn ブロックにはランタイムタスクスケジューリングが必要 |
| Cranelift バックエンド | より高速なコンパイル | ランタイム性能は LLVM に劣る、将来のオプションのバックエンドとして |
| 外部 LLVM runtime のリンク | LLVM 内蔵ランタイムを使用 | 不要な依存を導入する |

---

## 実装戦略

### フェーズ分け

#### フェーズ 1：基礎フレームワーク
- [ ] inkwell 依存を追加
- [ ] LLVM コンテキスト初期化実装（`context.rs`）
- [ ] 基本型マッピング実装（`types.rs`）

#### フェーズ 2：関数翻訳
- [ ] 関数宣言翻訳実装（`func.rs`）
- [ ] 基本命令翻訳実装（算術、制御フロー、呼び出し）（`translator.rs`）
- [ ] 値マッピング実装（`values.rs`）

#### フェーズ 3：所有権型翻訳
- [ ] `&T`/`&mut T` トークン実装（ゼロサイズ、コンパイル後消失）
- [ ] `ref T` 実装（ファットポインタ `{ i32*, T* }`）
- [ ] `?T` 実装（`{ i1, T }` タグ付き共用体）
- [ ] `List(T)` 実装（`{ T*, i64, i64 }`）
- [ ] Move セマンティクス追跡実装（デストラクタ挿入判定用）

#### フェーズ 4：spawn ブロックコード生成
- [ ] `spawn_placement.rs` の解析結果を消費
- [ ] 直接子式 → タスク関数生成
- [ ] 依存タスクのスケジューリングコード生成
- [ ] リソース競合の直列化
- [ ] spawn for 展開

#### フェーズ 5：FFI コード生成
- [ ] `native()` → `declare external`（`ffi.rs`）
- [ ] 引数マーシャリング / 戻り値アンマーシャリング
- [ ] 不透明型レイアウト（単一フィールド最適化を含む）
- [ ] `?T` null ポインタ最適化（FFI 専用）

#### フェーズ 6：デストラクタコード生成
- [ ] `.drop` バインディング識別
- [ ] スコープ終了クリーンアップ挿入（逆順）（`drop.rs`）
- [ ] 早期 return パスのクリーンアップ
- [ ] `?` エラー伝播パスのクリーンアップ

#### フェーズ 7：ランタイムライブラリリンク
- [ ] `runtime_task_spawn` / `runtime_task_wait_all` 等のランタイム関数実装
- [ ] ランタイム静的ライブラリをリンク
- [ ] エンドツーエンド統合テスト

### 依存関係

- RFC-024（spawn ブロック並行）→ フェーズ 4 の入力
- RFC-009 v9（所有権）→ フェーズ 3、6 の入力
- RFC-008（ランタイムアーキテクチャ）→ フェーズ 7 の入力
- RFC-026（FFI 機構）→ フェーズ 5 の入力

---

## 関連研究

### Lazy Task Creation (1990)[^1]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 著者 | James R. Larus, Robert H. Halstead Jr. |
| 核心 | 子タスクの遅延作成、オンデマンド作成 |
| 参考価値 | spawn ブロック内タスクのオンデマンドスケジューリングの理論的基礎 |

**核心思想**：タスクを即時作成せず、遅延作成する。親タスクが子タスクの値を必要とした時点で子タスクを作成する。これは細粒度並列タスクの性能オーバーヘッド問題を解決する[^1]。YaoXiang の spawn ブロックスケジューリングはこの思想を参考としている — タスクはコンパイル時に識別されるが、ランタイムでスレッドプールにオンデマンドでディスパッチされる。

### Lazy Scheduling (2014)[^2]

| 属性 | 説明 |
|------|------|
| 機関 | University of Maryland |
| 著者 | Tzannes, Caragea |
| 核心 | ランタイム適応スケジューリング、追加状態なし |
| 参考価値 | Full Runtime WorkStealing スケジューラ設計の参考 |

### SISAL 言語[^3]

| 属性 | 説明 |
|------|------|
| 機関 | Lawrence Livermore National Laboratory (LLNL) |
| 核心 | 単一代入言語、Dataflow グラフ、暗黙的並列 |
| 参考価値 | Dataflow モデルの産業レベル応用での実現可能性の証明 |

**重要な違い**：SISAL の並列性は**暗黙的** — 言語は単一代入セマンティクスで、コンパイラが全プログラムのデータ依存グラフを自動解析して並列性を決定する。YaoXiang の並列性は**明示的** — ユーザが `spawn {}` ブロックで並列領域をマークし、コンパイラは spawn ブロック内でのみ依存を解析する。これは SISAL の全プログラム解析の複雑さを回避しつつ、ユーザの並列挙動の制御を保持する。

### Mul-T 並列 Scheme[^4]

| 属性 | 説明 |
|------|------|
| 機関 | MIT |
| 核心 | Future 構造、Lazy Task Creation 実装 |
| 参考価値 | 具体的実装の参考 |

### 比較まとめ

| 技術 | 遅延作成 | 並列マーク | 解析範囲 | 所有権 |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | 暗黙 | 全プログラム | N/A |
| Lazy Scheduling[^2] | ✅ | 暗黙 | 全プログラム | N/A |
| SISAL[^3] | ✅ | 暗黙（単一代入） | 全プログラム | N/A |
| Mul-T[^4] | ✅ | 明示（future） | 呼び出し点 | N/A |
| **YaoXiang** | ✅ | **明示（spawn ブロック）** | **spawn ブロック内** | **✅（Move + トークン + ref）** |

**YaoXiang の革新点**：並列マークを「各関数呼び出し」（future）から「構造化ブロック」（spawn）へと昇格させた。ユーザは通常コードを書き、並列が必要な箇所に spawn ブロックを置く。解析範囲は spawn ブロック内に制約され、コンパイル効率が高く、振る舞いが制御可能。

---

## 付録

### 付録 A：Rust async との比較

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| コンパイル成果物 | ステートマシン + マシンコード | マシンコード + spawn タスクメタデータ |
| ランタイム | tokio | 静的リンクスケジューラ（~200-500KB） |
| 並行マーク | async/await キーワード | `spawn { }` ブロック |
| タスク作成 | コンパイル時にステートマシン生成 | コンパイル時に直接子式を識別 → タスク関数 |
| カラー関数 | async 感染 | **関数カラーリングなし** |
| 同期待機 | `.await` | spawn ブロックが自動同期ブロッキング |
| メモリ管理 | GC（ランタイム） | **RAII（決定論的）** |
| 共有機構 | `Arc::new()` + 手動 Weak | **`ref` キーワード（コンパイラが Rc/Arc を自動選択）** |

### 付録 B：設計決定記録

| 決定 | 内容 | 日付 |
|------|------|------|
| LLVM AOT 採用 | 直接 Codegen、過度な抽象化なし | 2026-02-15 |
| 並行モデルの整合 | RFC-024 spawn ブロック直接子式モデルに整合 | 2026-06-10 |
| DAG 解析範囲 | spawn ブロック内、spawn ブロックを跨がない（RFC-024 に整合） | 2026-06-05 |
| 所有権モデルの整合 | RFC-009 v9 に整合：`&T`/`&mut T` トークン + `ref` キーワード | 2026-06-10 |
| デュアルバックエンドモデル | VM（開発）+ LLVM（本番）、RFC-008 に整合 | 2026-05-11 |
| スケジューラ形態 | 静的ライブラリを exe にリンク、~200-500KB、GC なし | 2026-05-11 |
| FFI コード生成 | RFC-026 統合：`native()` declare + マーシャリング | 2026-06-10 |
| デストラクタ | `.drop` → RAII クリーンアップ挿入、RFC-026 §7 に整合 | 2026-06-10 |
| 副作用処理 | `@IO`/`@Pure` 推論を削除、RFC-024 リソース型に置換 | 2026-06-10 |
| リフレクションメタデータ | exe の .reflect セクションにコンパイル、mmap でオンデマンドロード | 2026-05-11 |
| 論文引用 | Lazy Task Creation 等を保持、YaoXiang との違いを明示 | 2026-02-16 |

---

## 参考文献

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT.

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland.

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory.

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024：spawn ブロックベースの並行モデル](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 並行モデルとスケジューラ疎結合設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-009：所有権モデル設計](../accepted/009-ownership-model.md)
- [RFC-026：FFI コア機構](./026-ffi-core-mechanism.md)

---

## ライフサイクルと帰趣

| 状態 | 位置 | 説明 |
|------|------|------|
| **ドラフト** | `docs/design/rfc/` | 著者の草稿、レビュー提出待ち |
| **レビュー中** | `docs/design/rfc/review/` | コミュニティの議論とフィードバックを公開 |
| **承認済み** | `docs/design/rfc/accepted/` | 正式な設計文書になる |
| **拒否** | `docs/design/rfc/` | RFC ディレクトリに保持 |

> 現在の状態：**レビュー中** — RFC-024 spawn ブロック並行モデル、RFC-009 v9 所有権モデル、RFC-026 FFI 機構に整合済み
```