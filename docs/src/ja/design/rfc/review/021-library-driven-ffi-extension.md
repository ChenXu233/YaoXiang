---
title: "RFC-021：ライブラリ駆動型FFI拡張と言語間呼び出しサポート"
---

# RFC-021：ライブラリ駆動型FFI拡張と言語間呼び出しサポート

> **ステータス**: 草案
> **作成者**: 晨煦
> **作成日**: 2026-03-14
> **最終更新**: 2026-05-29

> **参考**:
> - [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
> - [FFI実装計画](../reference/plan/completed/FFI.md)

## 概要

本ドキュメントは**ライブラリ駆動型**のFFI（外部関数インターフェース）拡張方案を提案する。FFIの唯一のエグゾースは `native("symbol")` 宣言と `FfiRegistry` ランタイムレジストリであり、コアには第二のメカニズムを導入しない。在此基础上通过标准库提供动态库加载、跨语言调用绑定等能力。具体语言的调用绑定（如 C、Python、JavaScript）由官方工具链自动生成或由各项目按需编写。

## 動機

### 既存実装の不足

現在のFFI実装は以下の能力を既に持っている：
- `native("symbol")` 構文による外部関数宣言
- `FfiRegistry` 関数レジストリ

しかし機能は比較的単一である：
- 動的ライブラリロードの 지원がない
- 言語間呼び出しのインフラがない
- 自動化されたバインディング生成ツールがない

### 設計哲学

YaoXiangは **「コアは簡潔、複雑さはライブラリに沈める」** の原則に従う：

> **良い品味 (Good Taste)**: 言語の責任はアトミックな能力を提供することであり、全てを網羅する機能セットではない。複雑さはライブラリで解決すべきであり、コンパイラに蓄積すべきではない。

因此、本方案：
- ✅ **ゼロ構文変更** — 完全な後方互換性、FFIエントリーポイントは `native("symbol")` のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで拡張
- ✅ **ツールチェーン自動化** — バインディングは `yx-bindgen` が自動生成、手作業ではない
- ✅ **プログレッシブエンハンスメント** — 開発者は必要に応じて機能を導入

## 提案

### 1. コアFFIライブラリの拡張

`std.ffi` モジュールを拡張する。注意点：外部関数への呼び出しはすべて相変わらず `native("symbol")` 宣言を通じて行われ、`std.ffi` は補助能力のみを提供する。

#### 1.1 動的ライブラリのロード

```yaoxiang
import ffi

# 動的ライブラリ (.so/.dll/.dylib) をロード
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、native で使用可能なシンボル名を返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols` はシンボル名を FfiRegistry の既知テーブルに登録する。その後、ユーザーは相変わらず `native` 宣言を通じて使用する：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文はなく、`try_call` ラッパーもない。

#### 1.2 ライブラリ管理

```yaoxiang
# ロード済みのライブラリを一覧表示
loaded = ffi.loaded_libraries()

# ライブラリのアンロード
ffi.unload_library(lib)

# ライブラリのバージョンチェック
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

言語間呼び出し規約と型変換は、ランタイムで汎用ラッパーとして処理するのではなく、`yx-bindgen` がコンパイル時に生成する。

### 2. 動的ライブラリロードの実装

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

`OsError` はプラットフォームネイティブのエラーコード（Linux の `dlerror()`、Windows の `GetLastError()`）を携带し、デバッグ可能性を確保する。

### 3. 多言語バインディング：ツールチェーン方案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という幻想を諦める。代わりに公式ツールチェーンが自動生成する。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                                │
│                                               │
│  // ユーザーは native 宣言のみを書く             │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時                   | ランタイム時
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (Cヘッダー → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディングジェネレーター (`yx-bindgen`)

`yx-bindgen` は独立したCLIツールで、CヘッダーファイルからYaoXiang FFIバインディングコードを生成する：

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

`yx-bindgen` は公式にメンテナンスされ、以下を保証する：
- 型マッピングの完全性（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体レイアウトのアライメント（自動 `#[repr(C)]` 等価物）
- コールバック署名の変換

#### 3.3 公式メンテナンスのバインディングパッケージ

YaoXiangコアチームは全ての言語の汎用バインディングライブラリのメンテナンスを約束しないが、公式の `libc` バインディングパッケージ（POSIX + Windows APIサブセット）をFFIベストプラクティス例および基礎能力として提供する。

他の言語やライブラリのバインディング：
- `yx-bindgen` で自行生成
- YaoXiangパッケージとして公開可能（`libsqlite3`、`libcurl`、`libsdl2` など）
- コアチームはメンテナンスを担当しないが、パブリッシュとバージョニングのメカニズムを提供する

### 4. 型変換層

#### 4.1 コンパイル時型マッピング

型変換はランタイムラッパーを通じて行わず、`yx-bindgen` 生成時に静的に決定する：

| C 型 | YaoXiang 型 | 変換方式 |
|--------|---------------|----------|
| `int` | `Int` | 直接値渡し |
| `char*` | `*const u8` | ポインタ渡し |
| `void*` | `*mut opaque` | 不透明ポインタ |
| `struct T` | `extern struct T` | メモリレイアウト一致 |
| `int*` | `*mut Int` | ポインタ渡し（可变） |
| `const int*` | `*const Int` | ポインタ渡し（読み取り専用） |

#### 4.2 手動変換（標準ライブラリヘルパー）

```yaoxiang
# 明示的変換
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. メモリ所有権モデル

#### 5.1 基本原則

FFI境界を越えるすべてのメモリ割当は、次の2つの問いに明確に答えなければならない：
1. **誰が割り当てたか？** （C側 `malloc` / YaoXiang側ランタイム）
2. **誰が解放するか？** （C側 `free` / YaoXiang側ランタイム）

`yx-bindgen` 生成時に、一般的なパターンにアノテーションを付ける：

```yaoxiang
# Cが割り当て、呼び出し側が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し側がポインタを割り当て
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

ランタイムはFFI境界を越えるポインタ参照の自動メモリ管理を行わない——所有権は明確に呼び出し側にある。

#### 5.2 文字列処理

C関数が返す `char*` は、YaoXiang `String` に変換される際に直ちにコピーされる。原ポインタの所有権はC関数が決める（アノテーションで宣言）、自動解放しない。

### 6. 安全性への考慮

#### 6.1 スレッド安全性

FFI関数呼び出しは**デフォルトでDAGスケジューリングに参加せず**、ブロッキング操作とみなす。reentrant であることが確認されたC関数は `@concurrent` でマークできる：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準Cライブラリ関数に対して可能な限りreentrancy情報をマークする（`strtok` の `_r` バリアントなど）。

**非同期呼び出し者への要求：** FFI関数を呼び出す前に、呼び出し側は対象の関数がreentrantであることを確認しなければならない。ランタイムは自動検出を行わない——これは静的に解決不可能な問題である。

#### 6.2 エラー隔離

- FFI呼び出しエラーは `Result` 型を通じて伝播（関数がResult返回型を宣言している場合）
- タイムアウトメカニズムで外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全

- ポインタ引数はYaoXiang側で `unsafe` マークが必要
- FFI境界を越えるポインタのライフタイムは呼び出し側が保証

### 7. コンパイラへの変更

**ゼロ構文変更** — `native("symbol")` 宣言のみでよく、現在のコンパイラ実装で既にサポートされている。

インタープリタ/ランタイムに追加：
- 動的ライブラリロード命令（`DynamicLibrary` のFFIバインディング）
- タイムアウトメカニズム

### 8. 許可されない機能

以下の機能は審査の結果、明確に除外され、RFCに含まれない：

- **`ffi.try_call`**: 冗長、既存の `native` + `Result` 返回型で十分
- **`ffi.verify_signature`**: ランタイムでコンパイラの仕事をしており、誤った抽象レベル
- **`ffi.async_call`**: reentrancy契約モデルが明確になってからお考えください
- **コミュニティメンテナンスバインディングテーブル**: 実行不可能、`yx-bindgen` ツールチェーン方案に変更

## トレードオフ

### 利点

- ✅ **ゼロ構文変更** — FFIエントリーポイントは `native("symbol")` のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリでプログレッシブに導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` がバインディング生成を自動処理
- ✅ **メモリ安全** — 所有権モデルが明確、自动回收によるuse-after-freeがない
- ✅ **デバッグ可能** — エラーはOSネイティブエラーコードを搭載

### 欠点

- ⚠️ 型安全はCヘッダーファイルの表現力に制限される（`void*` は静的区別不可）
- ⚠️ `yx-bindgen` はC標準の進化に対応するため継続的メンテナンスが必要
- ⚠️ 非C言語（Python/JS/Java）のバインディングは各プロジェクトが自行処理、统一案なし

## 実装戦略

### フェーズ1：コアライブラリ (v0.7)

- [ ] `std.ffi` モジュールを拡張
- [ ] `DynamicLibrary` 構造体を実装
- [ ] Linux/macOS 支持（`dlopen`/`dlsym`）
- [ ] Windows 支持（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウトメカニズムを追加
- [ ] ユニットテスト

### フェーズ2：yx-bindgen (v0.8)

- [ ] Cヘッダーファイルパーサーを実装（既存のClangバインディングまたは手書きパーサー）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言を生成
- [ ] 構造体レイアウトを生成
- [ ] 統合テスト：SQLite3、libcurlなどの実Cライブラリでバインディング生成

### フェーズ3：エコシステム基盤 (v0.9)

- [ ] 公式 `libc` バインディングパッケージをリリース（POSIX + Windows APIサブセット）
- [ ] バインディングパッケージ公開規範を制定
- [ ] ドキュメント：FFIベストプラクティス、メモリ所有権、スレッド安全性契約

## 他のRFCとの関係

- **RFC-001**: FFI呼び出しは外部関数として、デフォルト `@block`（DAGスケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI呼び出しは独立したタスク
- **RFC-020**: DAGにおけるFFIノードのスケジューリングセマンティクス、Phiノード、循环展開などのスケジューリングレベルの詳細設計

## オープン問題

- [ ] `yx-bindgen` をビルドシステム（`yaoxiang build`）に統合する必要があるか？
- [ ] WASMプラットフォームのFFIサポートはどのように設計するか？（WASMのインポート機構はdlopen完全不同）
- [ ] C++ name manglingを処理する `cxx-bindgen` を提供する必要があるか？（オプション、v1.0以降考虑）

---

## 付録A：設計意思決定記録

| 意思決定 | 決定 | 理由 | 日付 | 記録者 |
|------|------|------|------|--------|
| FFIエントリーポイント唯一化 | `native("symbol")` のみ | API分裂の回避 | 2026-05-29 | 晨煦 |
| `try_call` の除外 | 実装しない | 冗長、既存のResult型で十分 | 2026-05-29 | 晨煦 |
| `verify_signature` の除外 | 実装しない | ランタイムでコンパイラの仕事をしており | 2026-05-29 | 晨煦 |
| コミュニティバインディング → ツールチェーン | `yx-bindgen` 自動生成 | 実行不可能な幻想 | 2026-05-29 | 晨煦 |
| OSエラーコード | `FfiError` は必ず `os_error` を含む | デバッグ不可能なAPIは無用 | 2026-05-29 | 晨煦 |
| ゼロ構文変更 | ライブラリ実装に依存 | コア簡潔の原則 | 2026-03-14 | 晨煦 |
| 動的ライブラリロード | dlopen/dlsymを使用 | 標準OSインターフェース | 2026-03-14 | 晨煦 |
| エラー処理 | Result型を使用 | 一貫性 | 2026-03-14 | 晨煦 |

## 付録B：サンプルコード

### 完全な例：Cライブラリの使用

```yaoxiang
# C数学ライブラリをロード
libm = ffi.load_library("libm.so")

# Cシンボルをランタイムテーブルに登録（yx-bindgen がコンパイル時に実行）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native 宣言で使用
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
# sin_f / cos_f などは自動的に native("sin") / native("cos") として宣言済み
```

---

## 参考文献

- [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
- [FFI実装計画](../reference/plan/completed/FFI.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)