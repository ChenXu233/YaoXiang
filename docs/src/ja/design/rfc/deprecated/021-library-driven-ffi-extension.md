---
title: 'RFC-021: ライブラリ駆動 FFI 拡張と多言語呼び出しサポート'
status: '廃止済み'
author: '晨煦'
created: '2026-03-14'
updated: '2026-06-05（廃止）'
---

# RFC-021: ライブラリ駆動 FFI 拡張と多言語呼び出しサポート

> **⚠️ 廃止**：本ドキュメントは廃止済みであり、内容は
> [RFC-026: FFI コアメカニズム](../accepted/026-ffi-core-mechanism.md) に統合されました。

> **参考**:
>
> - [RFC-001: 並作モデルとエラーハンドリングシステム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ脱結合設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-026: FFI コアメカニズム](../accepted/026-ffi-core-mechanism.md)

## 概要

本ドキュメントは**ライブラリ駆動**の FFI（外部関数インターフェース）拡張方案を提案する。FFI の唯一の入口は
`native("symbol")` 宣言 + `FfiRegistry`
実行時レジストリであり、核心には第二のメカニズムを導入しない。これに基づき、標準ライブラリを通じて動的ライブラリのロード、言語間呼び出しバインディングなどの能力を提供する。具体的な言語の呼び出しバインディング（C、Python、JavaScript など）は、公式ツールチェーンにより自動生成されるか、各プロジェクトで必要に応じて記述される。

## 動機

### 既存実装の不足点

現在の FFI 実装は既に以下の能力を備えている：

- `native("symbol")` 構文による外部関数の宣言
- `FfiRegistry` 関数レジストリ

しかし、機能は比較的単一である：

- 動的ライブラリのロードサポートが欠如
- 言語間呼び出しのインフラが存在しない
- 自動化されたバインディング生成ツールが欠如

### 設計哲学

YaoXiang は **「核心はシンプルに、複雑性はライブラリに沈める」** という原則に従う：

> **良き趣味 (Good
> Taste)**：言語の職責はアトミックな能力を提供することであり、大かつ全の機能セットではない。複雑性はライブラリで解決すべきであり、コンパイラに堆積させるべきではない。

したがって、本方案では：

- ✅ **構文変更ゼロ** — 完全後方互換、FFI 入口は `native("symbol")` のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリを通じて拡張
- ✅ **ツールチェーン自動化** — バインディングは `yx-bindgen` により自動生成され、手動保守ではない
- ✅ **漸進的拡張** — 開発者は必要に応じて機能を導入

## 提案

### 1. 核心 FFI ライブラリの拡張

`std.ffi` モジュールを拡張する。注意：すべての外部関数呼び出しは依然として `native("symbol")`
宣言を経由し、`std.ffi` は補助能力のみを提供する。

#### 1.1 動的ライブラリのロード

```yaoxiang
import ffi

# 動的ライブラリをロード (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、native で利用可能な symbol 名を返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols`
はシンボル名を FfiRegistry の既知テーブルに登録する。その後、ユーザは依然として `native`
宣言を通じて使用する：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文はなく、`try_call` ラッパーもない。

#### 1.2 ライブラリ管理

```yaoxiang
# ロード済みライブラリを列挙
loaded = ffi.loaded_libraries()

# ライブラリをアンロード
ffi.unload_library(lib)

# ライブラリのバージョンチェック
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol 構造を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

言語間呼び出し規約と型変換は、実行時の汎用ラッパーでは処理されず、`yx-bindgen`
によりコンパイル時に生成される。

### 2. 動的ライブラリロードの実装

#### 2.1 核心データ構造

```rust
pub struct DynamicLibrary {
    handle: *mut std::ffi::c_void,
    path: String,
}

impl DynamicLibrary {
    pub fn load(path: &str) -> Result<Self, FfiError>;
    pub fn get_symbol(&self, name: &str) -> Result<*mut std::ffi::c_void, FfiError>;
    pub fn unload(self) -> Result<(), FfiError>;
}
```

#### 2.2 エラー型

```rust
pub enum FfiError {
    LibraryNotFound { name: String, os_error: Option<OsError> },
    SymbolNotFound { name: String, os_error: Option<OsError> },
    CallFailed { message: String, os_error: Option<OsError> },
    Timeout,
}

pub struct OsError {
    pub code: i32,
    pub message: String,
}
```

`OsError` はプラットフォームネイティブのエラーコード（Linux の `dlerror()`、Windows の
`GetLastError()`）を携帯し、デバッグ可能性を保証する。

### 3. 多言語バインディング：ツールチェーン方案

「各言語のコミュニティメンテナがバインディングライブラリを保守する」という幻想を捨てる。代わりに、公式ツールチェーンによる自動生成に変更する。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                              │
│                                               │
│  // ユーザは native 宣言のみを書く              │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時             | 実行時
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (C ヘッダ → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディングジェネレータ (`yx-bindgen`)

`yx-bindgen` は独立した CLI ツールであり、C ヘッダファイルから YaoXiang
FFI バインディングコードを生成する：

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3.yx
```

生成結果の例：

```yaoxiang
# 自動生成、手動編集禁止
# Source: /usr/include/sqlite3.h

sqlite3_open: (filename: *const u8, ppDb: *mut *mut opaque) -> Int
    = native("sqlite3_open")

sqlite3_close: (db: *mut opaque) -> Int
    = native("sqlite3_close")

sqlite3_exec: (
    db: *mut opaque,
    sql: *const u8,
    callback: *mut opaque,
    arg: *mut opaque,
    errmsg: *mut *mut u8,
) -> Int
    = native("sqlite3_exec")
```

`yx-bindgen` は公式により保守され、以下を保証する：

- 型マッピングの完全性（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体レイアウトのアラインメント（自動 `#[repr(C)]` 等価物）
- callback シグネチャ変換

#### 3.3 公式保守のバインディングパッケージ

YaoXiang コアチームはすべての言語の汎用バインディングライブラリを保守することは約束しないが、公式の
`libc` バインディングパッケージ（POSIX + Windows
API サブセット）を提供し、FFI ベストプラクティスの例と基礎能力とする。

その他の言語とライブラリのバインディング：

- `yx-bindgen` を使用して自行生成
- YaoXiang パッケージとして公開可能（例：`libsqlite3`、`libcurl`、`libsdl2`）
- コアチームは保守を担当しないが、パッケージの公開とバージョン管理メカニズムを提供

### 4. 型変換層

#### 4.1 コンパイル時の型マッピング

型変換は実行時ラッパーでは行われず、`yx-bindgen` の生成時に静的に決定される：

| C 型         | YaoXiang 型       | 変換方式                     |
| ------------ | ----------------- | ---------------------------- |
| `int`        | `Int`             | 直接値渡し                   |
| `char*`      | `*const u8`       | ポインタ渡し                 |
| `void*`      | `*mut opaque`     | 不透明ポインタ               |
| `struct T`   | `extern struct T` | メモリレイアウト一致         |
| `int*`       | `*mut Int`        | ポインタ渡し（可変）         |
| `const int*` | `*const Int`      | ポインタ渡し（読み取り専用） |

#### 4.2 手動変換（標準ライブラリ補助）

```yaoxiang
# 明示的変換
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. メモリオーナーシップモデル

#### 5.1 基本原則

FFI 境界を跨ぐすべてのメモリ割り当ては、二つの質問に明確に答えなければならない：

1. **誰が割り当てるか？** （C 側 `malloc` / YaoXiang 側実行時）
2. **誰が解放するか？** （C 側 `free` / YaoXiang 側実行時）

`yx-bindgen` 生成時、一般的なパターンにアノテーションを追加する：

```yaoxiang
# C が割り当て、呼び出し側が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し側がポインタを割り当て
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

実行時は FFI 境界を跨ぐポインタ参照に対し自動メモリ管理を行わない——所有権は明確に呼び出し側に帰属する。

#### 5.2 文字列処理

C 関数が返す `char*` は、YaoXiang の `String`
に変換される際に即座にコピーされる。元ポインタの所有権は C 関数が決定し（アノテーションで宣言）、自動解放されない。

### 6. 安全性考慮

#### 6.1 並行安全性

FFI 関数呼び出しは**デフォルトで DAG スケジューリングに参加せず**、ブロッキング操作と見なされる。reentrant であることが確認された C 関数は
`@concurrent` としてマーク可能：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準 C ライブラリ関数に対し可能な限り reentrancy 情報を标注する（`strtok` の `_r`
バリアントなど）。

**非同期呼び出し側への要求：**
FFI 関数を呼び出す前に、呼び出し側は対象関数が reentrant であることを保証しなければならない。実行時は自動検出を行わない——これは静的に解決不可能な問題である。

#### 6.2 エラー隔離

- FFI 呼び出しエラーは `Result` 型で伝播する（関数が Result 戻り型を宣言している場合）
- タイムアウト機構が外部関数のデッドロックを防止する

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry 層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全性

- ポインタ引数は YaoXiang 側の `unsafe` マーカーを必要とする
- FFI 境界を跨ぐポインタのライフタイムは呼び出し側が保証する

### 7. コンパイラの改変

**構文変更ゼロ** — `native("symbol")` 宣言のみが必要で、既に現在のコンパイラに実装済み。

インタプリタ/実行時に以下を追加：

- 動的ライブラリロード命令（`DynamicLibrary` の FFI バインディング）
- タイムアウト機構

### 8. 採用されない機能

以下の機能は審査後に明確に除外され、RFC に組み入れない：

- **`ffi.try_call`**：冗長であり、`native` + `Result` 戻り型が既に存在
- **`ffi.verify_signature`**：実行時にコンパイラの処理を行う、誤った抽象化レベル
- **`ffi.async_call`**：reentrancy 契約モデルの明確化後に再考する必要あり
- **コミュニティ保守バインディング表**：実行不可能であり、`yx-bindgen` ツールチェーン方案に変更

## トレードオフ

### 利点

- ✅ **構文変更ゼロ** — FFI 入口は `native("symbol")` のみ、完全後方互換
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリを通じて漸進的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` が自動的にバインディング生成を処理
- ✅ **メモリ安全** — 所有権モデルが明確、自動回収による use-after-free なし
- ✅ **デバッグ可能** — エラーは OS ネイティブエラーコードを携帯

### 欠点

- ⚠️ 型安全性は C ヘッダファイルの表現能力に制限される（`void*` は静的に区別不可）
- ⚠️ `yx-bindgen` は C 標準の進化に対応するため継続的な保守が必要
- ⚠️
  C 以外の言語（Python/JS/Java）のバインディングは各プロジェクトが自行処理する必要があり、統一方案なし

## 実装戦略

### フェーズ 1：核心ライブラリ (v0.7)

- [ ] `std.ffi` モジュールの拡張
- [ ] `DynamicLibrary` 構造の実装
- [ ] Linux/macOS のサポート（`dlopen`/`dlsym`）
- [ ] Windows のサポート（`LoadLibrary`/`GetProcAddress`）
- [ ] 実行時にタイムアウト機構を追加
- [ ] ユニットテスト

### フェーズ 2：yx-bindgen (v0.8)

- [ ] C ヘッダファイルパーサーの実装（既存の Clang バインディングまたは手書き parser に基づく）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言の生成
- [ ] 構造体レイアウトの生成
- [ ] 統合テスト：SQLite3、libcurl などの実際の C ライブラリに対するバインディング生成

### フェーズ 3：エコシステム基礎 (v0.9)

- [ ] 公式 `libc` バインディングパッケージの公開（POSIX + Windows API サブセット）
- [ ] バインディングパッケージ公開規範の策定
- [ ] ドキュメント：FFI ベストプラクティス、メモリオーナーシップ、並行安全性契約

## 他の RFC との関係

- **RFC-001**：FFI 呼び出しは外部関数として、デフォルトで
  `@block`（DAG スケジューリングに参加しない）
- **RFC-008**：スケジューラ脱結合設計、FFI 呼び出しは独立タスク
- **RFC-020**：DAG 内での FFI ノードのスケジューリングセマンティクス、Phi ノード、ループ展開などのスケジューリングレベルの詳細設計

## 未解決問題

- [ ] `yx-bindgen` をビルドシステム（`yaoxiang build`）に統合する必要があるか？
- [ ] WASM プラットフォームでの FFI サポートはどのように設計するか？（WASM のインポート機構は dlopen と全く異なる）
- [ ] C++ name mangling を処理するための `cxx-bindgen` を提供するか？（オプション、v1.0 以降検討）

---

## 付録 A：設計決定記録

| 決定                              | 結論                                | 理由                        | 日付       | 記録者 |
| --------------------------------- | ----------------------------------- | --------------------------- | ---------- | ------ |
| FFI 入口の唯一化                  | `native("symbol")` のみ保持         | API 分裂の回避              | 2026-05-29 | 晨煦   |
| `try_call` の除外                 | 実装しない                          | 冗長、Result 型が既に存在   | 2026-05-29 | 晨煦   |
| `verify_signature` の除外         | 実装しない                          | 実行時にコンパイラの処理    | 2026-05-29 | 晨煦   |
| コミュニティ保守 → ツールチェーン | `yx-bindgen` 自動生成               | 実行不可能な幻想            | 2026-05-29 | 晨煦   |
| OS エラーコード                   | `FfiError` は必ず `os_error` を含む | デバッグ不可能な API は無用 | 2026-05-29 | 晨煦   |
| 構文変更ゼロ                      | ライブラリ実装に依存                | 核心シンプル原則            | 2026-03-14 | 晨煦   |
| 動的ライブラリロード              | dlopen/dlsym を使用                 | 標準 OS インターフェース    | 2026-03-14 | 晨煦   |
| エラーハンドリング                | Result 型を使用                     | 一貫性                      | 2026-03-14 | 晨煦   |

## 付録 B：サンプルコード

### 完全な例：C ライブラリの使用

```yaoxiang
# C 数学ライブラリをロード
libm = ffi.load_library("libm.so")

# C シンボルを実行時テーブルに登録（yx-bindgen はコンパイル時にこれを行う）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native 宣言を通じて使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗する可能性のある C 関数を呼び出す際は Result を使用
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### yx-bindgen の使用

```bash
# すべての宣言を自動生成、手書き不要
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# YaoXiang でインポート
import "math_bindings.yx"
# sin_f / cos_f などは native("sin") / native("cos") として自動宣言済み
```

---

## 参考文献

- [RFC-001: 並作モデルとエラーハンドリングシステム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行モデルとスケジューラ脱結合設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-026: FFI コアメカニズム](../accepted/026-ffi-core-mechanism.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)
