---
title: "RFC-021：ライブラリ駆動型FFI拡張と異言語間呼び出しサポート"
---

# RFC-021: ライブラリ駆動型FFI拡張と異言語間呼び出しサポート

> **状態**: 草案
> **作者**: 晨煦
> **作成日**: 2026-03-14
> **最終更新**: 2026-05-29

> **参照**:
> - [RFC-001: 並作モデルと錯誤処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
> - [FFI実装計画](../reference/plan/completed/FFI.md)

## 概要

本文書は**ライブラリ駆動型**のFFI（外部関数インターフェース）拡張案を提案する。FFIの唯一のエンドポイントは`native("symbol")`宣言と`FfiRegistry`ランタイムレジストリであり、核心部分には第二のメカニズムを導入しない。その上に標準ライブラリを通じて動的ライブラリ読み込み、異言語間呼び出しバインディングなどの機能を提供する。具体的な言語の呼び出しバインディング（C、Python、JavaScriptなど）は公式ツールチェーンが自動生成するか、各プロジェクトがニーズに応じて作成する。

## 動機

### 既存実装の不足

現在のFFI実装は既に以下の能力を持つ：
- `native("symbol")`構文による外部関数宣言
- `FfiRegistry`関数レジストリ

しかし機能は比較的シンプル：
- 動的ライブラリ読み込みサポートの欠如
- 異言語間呼び出しのインフラストラクチャがない
- 自動バインディング生成ツールの欠如

### 設計哲学

YaoXiangは**「核心はシンプルに、複雑性はライブラリに落とす」**の原則に従う：

> **良い品味 (Good Taste)**: 言語の責任は原子能力を提供することであり、全てを網羅した機能セットを提供することではない。複雑性はライブラリで解決すべきであり、コンパイラに蓄積すべきではない。

したがって、本方案は以下の特徴を持つ：
- ✅ **構文変更ゼロ** — 完全な後方互換性、FFIエントーポイントは`native("symbol")`のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリを通じて拡張
- ✅ **ツールチェーン自動化** — バインディングは`yx-bindgen`が自動生成、手作業でのメンテナンス不要
- ✅ **プログレッシブエンハンス** — 開発者は必要に応じて機能を導入

## 提案

### 1. 核心FFIライブラリ拡張

`std.ffi`モジュールを拡張する。注意：外部関数への呼び出しはすべて引き続き`native("symbol")`宣言を通じて行われ、`std.ffi`は補助能力のみを提供する。

#### 1.1 動的ライブラリ読み込み

```yaoxiang
import ffi

# 動的ライブラリ読み込み (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、nativeで使用可能なシンボル名を返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library`は`DynamicLibrary`ハンドルを返し、`register_library_symbols`はシンボル名をFfiRegistryの既知テーブルに登録する。その後、ユーザーは引き続き`native`宣言を通じて使用する：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文はなく、`try_call`ラッパーもない。

#### 1.2 ライブラリ管理

```yaoxiang
# 読み込み済みライブラリのリスト
loaded = ffi.loaded_libraries()

# ライブラリのアンロード
ffi.unload_library(lib)

# ライブラリバージョン検査
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

異言語間呼び出し規約と型変換は、実行時に汎用ラッパーでは処理せず、`yx-bindgen`がコンパイル時に生成する。

### 2. 動的ライブラリ読み込み実装

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

#### 2.2 錯誤类型

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

`OsError`はプラットフォームネイティブのエラーコード（Linuxの`dlerror()`、Windowsの`GetLastError()`）を携带し、デバッグ可能性を保証する。

### 3. 多言語バインディング：ツールチェーン方案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という幻想を諦める。代わりに公式ツールチェーンが自動生成する。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                                │
│                                               │
│  // ユーザーはnative宣言のみを書く              │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時                   | 実行時
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (Cヘッダ → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディング生成器 (`yx-bindgen`)

`yx-bindgen`は独立したCLIツールで、CヘッダファイルからYaoXiang FFIバインディングコードを生成する：

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

`yx-bindgen`は公式メンテナンスされ、以下を保証する：
- 型マッピングの完全性（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体レイアウトの整合（自動`#[repr(C)]`同等物）
- コールバック署名変換

#### 3.3 公式メンテナンスのバインディングパッケージ

YaoXiangコアチームは全言語の汎用バインディングライブラリのメンテナンスを約束しないが、公式の`libc`バインディングパッケージ（POSIX + Windows APIサブセット）をFFI最佳事例と基礎能力として提供する。

他の言語とライブラリのバインディング：
- `yx-bindgen`を使用して自行生成
- YaoXiangパッケージとして公開可能（`libsqlite3`、`libcurl`、`libsdl2`など）
- コアチームはメンテナンスを担当しないが、パッケージの公開とバージョン管理メカニズムを提供する

### 4. 型変換層

#### 4.1 コンパイル時型マッピング

型変換は実行時ラッパーを通さず、`yx-bindgen`生成時に静的に決定される：

| C 型 | YaoXiang 型 | 変換方式 |
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

FFI境界を越えるすべてのメモリ確保は、以下の2つの問いに明確に回答する必要がある：
1. **誰が確保したか？** （C側`malloc` / YaoXiang側ランタイム）
2. **誰が解放するか？** （C側`free` / YaoXiang側ランタイム）

`yx-bindgen`生成時に、一般的なパターンに注釈を追加する：

```yaoxiang
# Cが確保し、呼び出し元が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し元がポインタを確保
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

実行時はFFI境界を越えるポインタ参照の自動メモリ管理を行わない——所有権は明確に呼び出し元にある。

#### 5.2 文字列処理

C関数が返す`char*`は、YaoXiangの`String`に変換される際に即座にコピーされる。元ポインタの所有権はC関数次第（註釈で宣言）であり、自動解放されない。

### 6. 安全性への配慮

#### 6.1 並行安全

FFI関数呼び出しは**デフォルトでDAGスケジューリングに参加せず**、ブロッキング操作とみなす。再入可能と確認されたC関数は`@concurrent`でマーク可能：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen`は標準Cライブラリ関数について再入可能性情報を可能な限りマークする（`strtok`の`_r`変種など）。

**非同期呼び出し元への要求：** FFI関数を呼び出す前に、呼び出し元は対象関数が再入可能であることを保証する必要がある。ランタイムは自動検出を行わない——これは静的解決不可能な問題である。

#### 6.2 錯誤隔離

- FFI呼び出しエラーは`Result`型を通じて伝播（関数がResult返回型を宣言している場合）
- タイムアウトメカニズムで外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全

- ポインタ引数はYaoXiang側で`unsafe`マークが必要
- FFI境界を越えるポインタのライフサイクルは呼び出し元が保証

### 7. コンパイラ変更

**構文変更ゼロ** — `native("symbol")`宣言のみが必要であり、現在のコンパイラ実装ですでに対応済み。

解釈器/ラン타임に追加：
- 動的ライブラリ読み込み命令（`DynamicLibrary`のFFIバインディング）
- タイムアウトメカニズム

### 8. 採用されない機能

以下の機能は審査を経て明確に除外され、RFCに含まれない：

- **`ffi.try_call`**: 冗長、`native` + `Result`返回型ですでに対応可能
- **`ffi.verify_signature`**: 実行時にコンパイラの仕事を行い 잘못った抽象レベル
- **`ffi.async_call`**: 再入可能性契約モデルが明確になってからねまり検討
- **コミュニティメンテナンスバインディングテーブル**: 実行不可能、`yx-bindgen`ツールチェーン方案に切り替え

## トレードオフ

### 利点

- ✅ **構文変更ゼロ** — FFIエントーポイントは`native("symbol")`のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリを通じてプログレッシブに導入
- ✅ **ツールチェーン駆動** — `yx-bindgen`がバインディング生成を自動処理
- ✅ **メモリ安全** — 所有権モデルが明確、自動回收によるuse-after-freeなし
- ✅ **デバッグ可能** — エラーはOSネイティブエラーコードを携带

### 欠点

- ⚠️ 型安全はCヘッダファイルの表現力に制限される（`void*`は静的区別不可）
- ⚠️ `yx-bindgen`はC標準の進化に追従するために継続的メンテナンスが必要
- ⚠️ 非C言語（Python/JS/Java）のバインディングは各プロジェクトが自行処理、统一方案なし

## 実装戦略

### フェーズ 1：核心ライブラリ (v0.7)

- [ ] `std.ffi`モジュールを拡張
- [ ] `DynamicLibrary`構造体を実装
- [ ] Linux/macOSをサポート（`dlopen`/`dlsym`）
- [ ] Windowsをサポート（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウトメカニズムを追加
- [ ] ユニットテスト

### フェーズ 2：yx-bindgen (v0.8)

- [ ] Cヘッダファイルパーサーを実装（既存のClangバインディングまたは手書きパーサー）
- [ ] 型マッピングシステムを実装
- [ ] `native("symbol")`宣言を生成
- [ ] 構造体レイアウトを生成
- [ ] 統合テスト：SQLite3、libcurlなど実際のCライブラリに対してバインディングを生成

### フェーズ 3：エコシステム基盤 (v0.9)

- [ ] 公式`libc`バインディングパッケージを公開（POSIX + Windows APIサブセット）
- [ ] バインディングパッケージ公開規範を制定
- [ ] ドキュメント：FFI最佳実践、メモリ所有権、並行安全契約

## 他のRFCとの関係

- **RFC-001**: FFI呼び出しは外部関数として、デフォルト`@block`（DAGスケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI呼び出しは独立タスク
- **RFC-020**: 動的モジュールとFFI統合設計

## 未解決問題

- [ ] `yx-bindgen`はビルドシステム（`yaoxiang build`）に統合する必要があるか？
- [ ] WASMプラットフォームのFFIサポートはどう設計するか？（WASMのインポートメカニズムはdlopen完全不同）
- [ ] C++ name manglingを処理する`cxx-bindgen`を提供する必要があるか？（オプション、v1.0後に検討）

---

## 付録 A：設計決定記録

| 決定 | 結論 | 理由 | 日付 | 記録者 |
|------|------|------|------|--------|
| FFIエントポイント統一化 | `native("symbol")`のみ | API分裂回避 | 2026-05-29 | 晨煦 |
| `try_call`除外 | 実装しない | 冗長、Result型ですでに対応 | 2026-05-29 | 晨煦 |
| `verify_signature`除外 | 実装しない | 実行時にコンパイラの仕事 | 2026-05-29 | 晨煦 |
| コミュニティバインディング → ツールチェーン | `yx-bindgen`自動生成 | 実行不可能な幻想 | 2026-05-29 | 晨煦 |
| OSエラーコード | `FfiError`は必ず`os_error`を携带 | デバッグ不能なAPIは無用 | 2026-05-29 | 晨煦 |
| 構文変更ゼロ | ライブラリ実装に依存 | 核心シンプル原則 | 2026-03-14 | 晨煦 |
| 動的ライブラリ読み込み | dlopen/dlsymを使用 | 標準OSインターフェース | 2026-03-14 | 晨煦 |
| 錯誤処理 | Result型を使用 | 一貫性 | 2026-03-14 | 晨煦 |

## 付録 B：サンプルコード

### 完全サンプル：Cライブラリの使用

```yaoxiang
# C数学ライブラリを読み込み
libm = ffi.load_library("libm.so")

# Cシンボルをランタイムテーブルに登録（yx-bindgenがコンパイル時にこれを行う）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native宣言を通じて使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗する可能性のあるC関数を呼び出す際にResultを使用
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### yx-bindgenの使用

```bash
# すべての宣言を自動生成、手書き不要
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# YaoXiangでインポート
import "math_bindings.yx"
# sin_f / cos_fなどは自動的にnative("sin") / native("cos")として宣言済み
```

---

## 参考文献

- [RFC-001: 並作モデルと錯誤処理システム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
- [FFI実装計画](../reference/plan/completed/FFI.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)