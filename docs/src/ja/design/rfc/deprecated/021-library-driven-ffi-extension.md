---
title: 'RFC-021: ライブラリ駆動型FFI拡張と異言語呼び出しサポート'
status: '廃棄'
author: '晨煦'
created: '2026-03-14'
updated: '2026-06-05（廃棄）'
---

# RFC-021: ライブラリ駆動型FFI拡張と異言語呼び出しサポート

> **⚠️ 廃棄**: 本ドキュメントは廃棄されました。内容は [RFC-026: FFIコア機構](../accepted/026-ffi-core-mechanism.md) に統合されました。

> **参考**:
>
> - [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime並行モデルとスケジューラ分離設計](../accepted/008-runtime-concurrency-model.md)
> - [RFC-026: FFIコア機構](../accepted/026-ffi-core-mechanism.md)

## 概要

本ドキュメントは**ライブラリ駆動型**のFFI（外部関数インターフェース）拡張方案を提案します。FFIの唯一のエグザンブルは `native("symbol")` 宣言 +
`FfiRegistry`
ランタイムレジストリであり、コアには第二の機構を導入しません。在此基础上通过标准库提供动态库加载、跨语言调用绑定等能力。具体语言的调用绑定（如 C、Python、JavaScript）由官方工具链自动生成或由各项目按需编写。

## 動機

### 既存実装の不足

現在のFFI実装は以下の能力を既に持っています：

- `native("symbol")` 構文で外部関数を宣言
- `FfiRegistry` 関数レジストリ

しかし機能が比較的単一です：

- 動的ライブラリの読み込みサポートがない
- 異言語呼び出しのインフラがない
- 自動化されたバインディング生成ツールがない

### 設計哲学

YaoXiangは **「コアは簡潔で、複雑さはライブラリに沈める」** の原則に従います：

> **良い好み (Good
> Taste)**: 言語の責務はアトミックな能力を提供することであり、多機能な機能セットを提供することではない。複雑さはライブラリで解決すべきであり、コンパイラに堆积すべきではない。

因此、本方案：

- ✅ **ゼロ構文変更** — 完全な後方互換性、FFIエグザンブルは `native("symbol")` のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリを通じて拡張
- ✅ **ツールチェーンの自動化** — バインディングは `yx-bindgen` が自動生成、手作業での維持ではない
- ✅ **漸進的強化** — 開発者は必要に応じて機能を導入

## 提案

### 1. コアFFIライブラリの強化

`std.ffi` モジュールを拡張します。注意：すべての外部関数呼び出しは引き続き `native("symbol")` 宣言を通じて行われ、`std.ffi`
は補助能力のみを提供します。

#### 1.1 動的ライブラリの読み込み

```yaoxiang
import ffi

# 動的ライブラリ (.so/.dll/.dylib) の読み込み
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、native が使用可能なシンボル名として登録
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols`
はシンボル名を FfiRegistry の既知テーブルに登録します。その後、ユーザーは引き続き `native` 宣言を使用して：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文はなく、`try_call` ラッパーもない。

#### 1.2 ライブラリ管理

```yaoxiang
# 読み込み済みライブラリのリスト
loaded = ffi.loaded_libraries()

# ライブラリのアンロード
ffi.unload_library(lib)

# ライブラリバージョンのチェック
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

異言語呼び出し規約と型変換は、実行時に汎用ラッパーで処理するのではなく、`yx-bindgen` がコンパイル時に生成します。

### 2. 動的ライブラリ読み込みの実装

#### 2.1 コアデータ構造

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

#### 2.2 エラータイプ

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

`OsError` はプラットフォームネイティブのエラーコード（Linux の `dlerror()`、Windows の `GetLastError()`）を携带し、デバッグ可能であることを保証します。

### 3. 多言語バインディング：ツールチェーン方案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という妄想を諦めます。代わりに公式ツールチェーンが自動生成します。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                               │
│                                               │
│  // ユーザーは native 宣言のみ書く               │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時                   | 実行時
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (Cヘッダ → .yx)  │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディング生成器 (`yx-bindgen`)

`yx-bindgen` は独立したCLIツールで、CヘッダファイルからYaoXiang FFIバインディングコードを生成します：

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

`yx-bindgen` は公式がメンテナンスし、以下を保証します：

- 型マッピングの完全性（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体レイアウトのアライメント（自動 `#[repr(C)]` 等価物）
- callback署名の変換

#### 3.3 公式メンテナンスのバインディングパッケージ

YaoXiangコアチームはすべての言語の汎用バインディングライブラリのメンテナンスを約束しませんが、FFI最佳実践の例と基础能力として、公式の `libc` バインディングパッケージ（POSIX + Windows APIサブセット）を提供します。

他の言語とライブラリのバインディング：

- `yx-bindgen` を使用して自行生成
- YaoXiangパッケージとして公開可能（`libsqlite3`、`libcurl`、`libsdl2` など）
- コアチームはメンテナンスの責任を負いませんが、パブリッシュとバージョン管理の機構を提供します

### 4. 型変換層

#### 4.1 コンパイル時の型マッピング

型変換は実行時ラッパー経由ではなく、`yx-bindgen` 生成時に静的に決定されます：

| C 型          | YaoXiang 型       | 変換方式           |
| ------------- | ----------------- | ------------------ |
| `int`         | `Int`             | 直接値渡し         |
| `char*`       | `*const u8`       | ポインタ渡し       |
| `void*`       | `*mut opaque`     | 不透明ポインタ     |
| `struct T`    | `extern struct T` | メモリレイアウト整合|
| `int*`        | `*mut Int`        | ポインタ渡し（可変）|
| `const int*`  | `*const Int`      | ポインタ渡し（読取専用）|

#### 4.2 手動変換（標準ライブラリ補助）

```yaoxiang
# 明示的変換
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. メモリ所有権モデル

#### 5.1 基本原則

FFI境界を跨ぐすべてのメモリ割り当ては、以下の2つの質問に明確に答える必要があります：

1. **誰が分配したか？** （C側 `malloc` / YaoXiang側ランタイム）
2. **誰が解放するか？** （C側 `free` / YaoXiang側ランタイム）

`yx-bindgen` 生成時に、一般的なパターンに注釈を追加します：

```yaoxiang
# Cが分配、呼び出し側が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し側がポインタを分配
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

実行時はFFI境界を跨ぐポインタ参照の自動メモリ管理を行いません——所有権は明確に呼び出し側に帰属します。

#### 5.2 文字列処理

C関数が返す `char*` は、YaoXiang `String`
に変換される際に直ちにコピーされます。元ポインタの所有権はC関数が決めます（注釈で宣言）、自動解放しません。

### 6. 安全性への配慮

#### 6.1 並行安全性

FFI関数呼び出しは**デフォルトでDAGスケジューリングに参加せず**、ブロッキング操作とみなします。再入可能と確認されたC関数には `@concurrent` をマークできます：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準Cライブラリ関数に可能な限り再入情報をマークします（`strtok` の `_r` バリアントなど）。

**非同期呼び出し側への要件：**
FFI関数を呼び出す前に、呼び出し側は対象関数が再入可能であることを確認する必要があります。実行時は自動検出を行いません——これは静的に解決できない問題です。

#### 6.2 エラー隔離

- FFI呼び出しエラーは `Result` 型を通じて伝播（関数がResult返回型を宣言している場合）
- タイムアウト機構で外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全

- ポインタ引数にはYaoXiang側の `unsafe` マークが必要
- FFI境界を跨ぐポインタのライフタイムは呼び出し側が保証

### 7. コンパイラの改変

**ゼロ構文変更** — `native("symbol")` 宣言のみ必要、現在のコンパイラ実装で既にサポート。

インタプリタ/ラン타임に追加：

- 動的ライブラリ読み込み命令（`DynamicLibrary` のFFIバインディング）
- タイムアウト機構

### 8. 採用しない機能

以下の機能は審査の結果として明確に除外され、本RFCに含めません：

- **`ffi.try_call`**: 不要、既存の `native` + `Result` 返回型で十分
- **`ffi.verify_signature`**: 実行時にコンパイラの仕事をさせるのは間違った抽象レベル
- **`ffi.async_call`**: 再入契約モデルが明確になるまで待つ必要あり
- **コミュニティメンテナンスのバインディングテーブル**: 実行不可能、`yx-bindgen` ツールチェーン方案に改める

## トレードオフ

###  長所

- ✅ **ゼロ構文変更** — FFIエグザンブルは `native("symbol")` のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリを通じて漸進的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` がバインディング生成を自動処理
- ✅ **メモリ安全** — 所有権モデルが明確、自動回收によるuse-after-freeなし
- ✅ **デバッグ可能** — エラーはOSネイティブエラーコードを携带

### 短所

- ⚠️ 型安全はCヘッダファイルの表現力に制限される（`void*` は静的に区別できない）
- ⚠️ `yx-bindgen` はC標準の進化に追従するために継続的なメンテナンスが必要
- ⚠️ C以外言語（Python/JS/Java）のバインディングは各プロジェクトが自行処理、统一方案なし

## 実装戦略

### フェーズ1：コアライブラリ (v0.7)

- [ ] `std.ffi` モジュールを拡張
- [ ] `DynamicLibrary` 構造体を実装
- [ ] Linux/macOSをサポート（`dlopen`/`dlsym`）
- [ ] Windowsをサポート（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウト機構を追加
- [ ] ユニットテスト

### フェーズ2：yx-bindgen (v0.8)

- [ ] Cヘッダファイルパーサー実装（既存のClangバインディングまたは手書きパーサー）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言を生成
- [ ] 構造体レイアウトを生成
- [ ] 統合テスト：SQLite3、libcurlなどの реальный Cライブラリに対してバインディングを生成

### フェーズ3：エコシステム基盤 (v0.9)

- [ ] 公式 `libc` バインディングパッケージを公開（POSIX + Windows APIサブセット）
- [ ] バインディングパッケージ公開規範の制定
- [ ] ドキュメント：FFI最佳実践、メモリ所有権、並行安全契約

## 他のRFCとの関係

- **RFC-001**: FFI呼び出しは外部関数として、デフォルト `@block`（DAGスケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI呼び出しは独立したタスク
- **RFC-020**: DAGにおけるFFIノードのスケジューリングセマンティクス、Phiノード、循环展開などのスケジューリングレベルの詳細設計

## 公開質問

- [ ] `yx-bindgen` はビルドシステム（`yaoxiang build`）に統合する必要がありますか？
- [ ] WASMプラットフォームのFFIサポートはどのように設計しますか？（WASMのインポート機構はdlopen完全不同）
- [ ] C++ name manglingを処理する `cxx-bindgen` を提供する必要がありますか？（オプション、v1.0以降で検討）

---

## 付録 A：設計決定記録

| 決定                    | 決定内容                       | 理由                    | 日付        | 記録者 |
| ----------------------- | ------------------------------ | ----------------------- | ----------- | ------ |
| FFIエグザンブルの唯一化 | `native("symbol")` のみ保持    | API分裂を避けるため     | 2026-05-29  | 晨煦   |
| `try_call` の除外       | 実装しない                     | 不要、Result型が存在    | 2026-05-29  | 晨煦   |
| `verify_signature` の除外 | 実装しない                   | 実行時にコンパイラの仕事をさせる | 2026-05-29  | 晨煦   |
| コミュニティバインディング → ツールチェーン | `yx-bindgen` 自動生成 | 実行不可能な妄想       | 2026-05-29  | 晨煦   |
| OSエラーコード          | `FfiError` は `os_error` を必須 | デバッグ不能なAPIは無用 | 2026-05-29  | 晨煦   |
| ゼロ構文変更            | ライブラリ実装に依存           | コア簡潔の原則          | 2026-03-14  | 晨煦   |
| 動的ライブラリ読み込み  | dlopen/dlsym を使用            | 標準OSインターフェース  | 2026-03-14  | 晨煦   |
| エラー処理              | Result 型を使用                | 一貫性                  | 2026-03-14  | 晨煦   |

## 付録 B：サンプルコード

### 完全な例：Cライブラリの使用

```yaoxiang
# C数学ライブラリの読み込み
libm = ffi.load_library("libm.so")

# Cシンボルをランタイムテーブルに登録（yx-bindgenがコンパイル時に実行）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native 宣言で関数を使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗する可能性のあるC関数を呼び出す場合はResultを使用
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### yx-bindgen の使用

```bash
# すべての宣言を自動生成、手書き不要
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# YaoXiang でインポート
import "math_bindings.yx"
# sin_f / cos_f などは自動的で native("sin") / native("cos") として宣言済み
```

---

## 参考文献

- [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime並行モデルとスケジューラ分離設計](../accepted/008-runtime-concurrency-model.md)
- [RFC-026: FFIコア機構](../accepted/026-ffi-core-mechanism.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)