---
title: "RFC-021: ライブラリ駆動型FFI拡張と異言語呼び出しサポート"
status: "廃止"
author: "晨煦"
created: "2026-03-14"
updated: "2026-06-05（廃止）"
---

# RFC-021: ライブラリ駆動型FFI拡張と異言語呼び出しサポート

> **⚠️ 廃止**: 本ドキュメントは廃止されました。内容は [RFC-026: FFIコアメカニズム](./026-ffi-core-mechanism.md) に統合されました。

> **参考**:
> - [RFC-001: 並列実行モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime同時実行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
> - [FFI実装計画](../reference/plan/completed/FFI.md)

## 概要

本ドキュメントは**ライブラリ駆動型**のFFI（外部関数インターフェース）拡張方案を提案します。FFIの唯一のエントリーポイントは `native("symbol")` 宣言と `FfiRegistry` ランタイムレジストリとし、コアには第二のメカニズムを導入しません。この上に標準ライブラリを通じて動的ライブラリロード、異言語呼び出しバインディングなどの能力を提供します。具体的な言語の呼び出しバインディング（C、Python、JavaScriptなど）は公式ツールチェーンにより自動生成、または各プロジェクトが必要に応じて記述します。

## 動機

### 既存実装の不足

現在のFFI実装は以下の能力を既に備えています：
- `native("symbol")` 構文による外部関数宣言
- `FfiRegistry` 関数レジストリ

しかし機能が比較的限定的です：
- 動的ライブラリロードのサポートがない
- 異言語呼び出しのインフラがない
- 自動化されたバインディング生成ツールがない

### 設計哲学

YaoXiangは **「コアはシンプルに、複雑さはライブラリに落とす」** の原則に従います：

> **良い品味 (Good Taste)**: 言語の責務は原子的な能力を提供することであり、全てを備えた機能セットではない。複雑さはライブラリで解決すべきであり、コンパイラに堆积すべきではない。

したがって、本方案は：
- ✅ **ゼロ構文変更** — 完全な後方互換性、FFIエントリーポイントは `native("symbol")` のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで拡張
- ✅ **ツールチェーン自動化** — バインディングは `yx-bindgen` が自動生成、手作業での維持ではない
- ✅ **漸進的強化** — 開発者は必要に応じて機能を導入

## 提案

### 1. コアFFIライブラリ強化

`std.ffi` モジュールを拡張します。注意：外部関数への呼び出しはすべて引き続き `native("symbol")` 宣言を通じて行われ、`std.ffi` は補助能力のみを提供します。

#### 1.1 動的ライブラリロード

```yaoxiang
import ffi

# 動的ライブラリをロード (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、nativeが使えるsymbol名を返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols` はシンボル名をFfiRegistryの既知テーブルに登録します。その後、ユーザーは引き続き `native` 宣言で使用します：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文はなく、`try_call` ラッパーもありません。

#### 1.2 ライブラリ管理

```yaoxiang
# ロード済みのライブラリを一覧表示
loaded = ffi.loaded_libraries()

# ライブラリのアンロード
ffi.unload_library(lib)

# ライブラリバージョン確認
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

異言語呼び出しの約束事と型変換は、ランタイムで汎用ラッパー経由で処理するのではなく、`yx-bindgen` がコンパイル時に生成します。

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

`OsError` はプラットフォームのネイティブエラーコード（Linuxの `dlerror()`、Windowsの `GetLastError()`）を携带し、デバッグ可能性を保証します。

### 3. 多言語バインディング：ツールチェーン方案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という幻想を諦めます。代わりに公式ツールチェーンで自動生成します。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiangコード                               │
│                                               │
│  // ユーザーはnative宣言のみを書く              │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時                   | ランタイム
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (Cヘッダー → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディングジェネレータ (`yx-bindgen`)

`yx-bindgen` は独立したCLIツールで、CヘッダーファイルからYaoXiang FFIバインディングコードを生成します：

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

`yx-bindgen` は公式にメンテナンスされ、以下を保証します：
- 型マッピングの完全性（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体のレイアウト整列（自動的な `#[repr(C)]` 等価物）
- コールバック署名の変換

#### 3.3 公式メンテナンスのバインディングパッケージ

YaoXiangコアチームは全言語の汎用バインディングライブラリのメンテナンスを約束しませんが、公式の `libc` バインディングパッケージ（POSIX + Windows APIサブセット）をFFIベストプラクティスの例と基本能力として提供します。

他の言語とライブラリのバインディング：
- `yx-bindgen` で自行生成
- YaoXiangパッケージとして公開可能（`libsqlite3`、`libcurl`、`libsdl2` など）
- コアチームはメンテナンスの責務を負わないが、パッケージの公開とバージョン管理メカニズムを提供

### 4. 型変換層

#### 4.1 コンパイル時型マッピング

型変換はランタイムラッパー経由ではなく、`yx-bindgen` 生成時に静的に決定されます：

| C型 | YaoXiang型 | 変換方式 |
|--------|---------------|----------|
| `int` | `Int` | 直接値渡し |
| `char*` | `*const u8` | ポインタ渡し |
| `void*` | `*mut opaque` | 不透明ポインタ |
| `struct T` | `extern struct T` | メモリレイアウト整合 |
| `int*` | `*mut Int` | ポインタ渡し（可変） |
| `const int*` | `*const Int` | ポインタ渡し（読み取り専用） |

#### 4.2 手動変換（標準ライブラリ補助）

```yaoxiang
# 明示的変換
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. メモリ所有権モデル

#### 5.1 基本原則

FFI境界を跨ぐすべてのメモリ割当てにおいて、以下の2つの問題に明示的に答える必要があります：
1. **誰が割当てたか？** （C側 `malloc` / YaoXiang側ランタイム）
2. **誰が解放するか？** （C側 `free` / YaoXiang側ランタイム）

`yx-bindgen` 生成時に、一般的なパターンに注釈を追加します：

```yaoxiang
# Cが割当て、呼び出し側が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し側がポインタを割当て
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

ランタイムはFFI境界を跨ぐポインタ参照の自動メモリ管理を行いません——所有権は明確に呼び出し側に帰属します。

#### 5.2 文字列処理

C関数が返す `char*` は、YaoXiangの `String` に変換される瞬間に即座にコピーされます。元ポインタの所有権はC関数次第（註釈で宣言）で、自動解放しません。

### 6. セキュリティ上の考慮事項

#### 6.1 並行安全性

FFI関数呼び出しは**デフォルトでDAGスケジューリングに参加せず**、ブロッキング操作と見なします。再入可能确定为なC関数には `@concurrent` マークを付けることができます：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準Cライブラリ関数について可能な限りの再入可能性情報をマークします（`strtok` の `_r` 変種など）。

**非同期呼び出し側の要件：** FFI関数を呼び出す前に、呼び出し側は対象関数が再入可能であることを確認する必要があります。ランタイムは自動検出を行いません——これは静的に解決不可能な問題です。

#### 6.2 エラー隔離

- FFI呼び出しエラーは `Result` 型を通じて伝播（関数がResult戻り値型を宣言している場合）
- タイムアウトメカニズムで外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全

- ポインタ引数にはYaoXiang側の `unsafe` マークが必要
- FFI境界を跨ぐポインタのライフタイムは呼び出し側が保証

### 7. コンパイラの変更

**ゼロ構文変更** — `native("symbol")` 宣言のみで良く、現在のコンパイラ実装で既にサポートされています。

解釈器/ラン타임への追加：
- 動的ライブラリロード命令（`DynamicLibrary` のFFIバインディング）
- タイムアウトメカニズム

### 8. 承認されない機能

以下の機能は審査の結果、明確に除外され、RFCに含めません：

- **`ffi.try_call`**: 冗長、`native` + `Result` 戻り値型が既に存在
- **`ffi.verify_signature`**: ランタイムでコンパイラの仕事をしており、誤った抽象レベル
- **`ffi.async_call`**: 再入可能性契約モデルが明確にってから検討要
- **コミュニティメンテナンスバインディングテーブル**: 実行不可能、`yx-bindgen` ツールチェーン方案に切り替え

## トレードオフ

### メリット

- ✅ **ゼロ構文変更** — FFIエントリーポイントは `native("symbol")` のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで漸進的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` がバインディング生成を自動処理
- ✅ **メモリ安全** — 所有権モデルが明確、自动回収によるuse-after-freeなし
- ✅ **デバッグ可能** — エラーにはOSのネイティブエラーコードが含まれる

### デメリット

- ⚠️ 型安全はCヘッダーファイルの表現力 LIMIT に制限される（`void*` は静的区別不可）
- ⚠️ `yx-bindgen` はC標準の進化に追従するために継続的なメンテナンスが必要
- ⚠️ 非C言語（Python/JS/Java）のバインディングは各プロジェクトで自行処理、统一方案なし

## 実装策略

### フェーズ1：コアライブラリ (v0.7)

- [ ] `std.ffi` モジュールを拡張
- [ ] `DynamicLibrary` 構造を実装
- [ ] Linux/macOSをサポート (`dlopen`/`dlsym`)
- [ ] Windowsをサポート (`LoadLibrary`/`GetProcAddress`)
- [ ] ランタイムにタイムアウトメカニズムを追加
- [ ] ユニットテスト

### フェーズ2：yx-bindgen (v0.8)

- [ ] Cヘッダーファイルパーサーを実装（既存のClangバインディングまたは自作パーサー）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言を生成
- [ ] 構造体レイアウトを生成
- [ ] 統合テスト：SQLite3、libcurlなどの実際のCライブラリに対してバインディングを生成

### フェーズ3：エコシステム基盤 (v0.9)

- [ ] 公式 `libc` バインディングパッケージを公開（POSIX + Windows APIサブセット）
- [ ] バインディングパッケージ公開規範を制定
- [ ] ドキュメント：FFIベストプラクティス、メモリ所有権、並行安全契約

## 他のRFCとの関係

- **RFC-001**: FFI呼び出しは外部関数として、デフォルトで `@block`（DAGスケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI呼び出しは独立したタスク
- **RFC-020**: DAGにおけるFFIノードのスケジューリングセマンティクス、Phiノード、循環展開などのスケジューリングレベルの詳細設計

## 開放問題

- [ ] `yx-bindgen` はビルドシステム（`yaoxiang build`）に統合する必要があるか？
- [ ] WASMプラットフォームのFFIサポートはどう設計するか？（WASMのインポートメカニズムはdlopenと全く異なる）
- [ ] C++名前マングリングを処理する `cxx-bindgen` を提供する必要があるか？（オプション、v1.0後に検討）

---

## 付録A：設計決定記録

| 決定 | 決定内容 | 理由 | 日付 | 記録者 |
|------|------|------|------|--------|
| FFIエントリーポイントの一元化 | `native("symbol")` のみ保留 | API分裂を避けるため | 2026-05-29 | 晨煦 |
| `try_call` の除外 | 実装しない | 冗長、Result型が既に存在 | 2026-05-29 | 晨煦 |
| `verify_signature` の除外 | 実装しない | ランタイムでコンパイラの仕事をしており | 2026-05-29 | 晨煦 |
| コミュニティバインディング → ツールチェーン | `yx-bindgen` 自動生成 | 実行不可能な幻想 | 2026-05-29 | 晨煦 |
| OSエラーコード | `FfiError` には `os_error` が必須 | デバッグ不能なAPIは無用 | 2026-05-29 | 晨煦 |
| ゼロ構文変更 | ライブラリ実装に依存 | コアシンプル原則 | 2026-03-14 | 晨煦 |
| 動的ライブラリロード | dlopen/dlsymを使用 | 標準OSインターフェース | 2026-03-14 | 晨煦 |
| エラー処理 | Result型を使用 | 一貫性 | 2026-03-14 | 晨煦 |

## 付録B：サンプルコード

### 完全示例：Cライブラリの使用

```yaoxiang
# C数学ライブラリをロード
libm = ffi.load_library("libm.so")

# Cシンボルをランタイムテーブルに登録（yx-bindgenがコンパイル時に実行）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native宣言で使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗する可能性のあるC関数を呼び出す場合はResultを使用
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### yx-bindgenの使用

```bash
# すべての宣言を自動生成、手書き不要
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# YaoXiangでインポート
import "math_bindings.yx"
# sin_f / cos_f などは自動的に native("sin") / native("cos") として宣言済み
```

---

## 参考文献

- [RFC-001: 並列実行モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime同時実行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
- [FFI実装計画](../reference/plan/completed/FFI.md)
- [Python ctypesドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)