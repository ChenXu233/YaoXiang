# RFC-021: ライブラリ主導 FFI 拡張と跨言語呼び出しサポート

> **状態**: 草案
> **著者**: 晨煦
> **作成日**: 2026-03-14
> **最終更新**: 2026-05-29

> **参考**:
> - [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
> - [FFI 実装計画](../reference/plan/completed/FFI.md)

## 概要

本書は**ライブラリ主導**の FFI（外部関数インターフェース）拡張案を提案する。FFI の唯一のエントリーポイントは `native("symbol")` 宣言と `FfiRegistry` ランタイムレジストリであり、コアには第二のメカニズムを導入しない。在此基础上通过标准库提供动态库加载、跨语言调用绑定等能力。具体语言的调用绑定（如 C、Python、JavaScript）由官方工具链自动生成或由各项目按需编写。

## 動機

### 既存実装の不足

現在の FFI 実装は既に以下の能力を持つ：

- `native("symbol")` 構文による外部関数宣言
- `FfiRegistry` 関数レジストリ

しかし機能が比較的单一である：

- 動的ライブラリ読み込みのサポートがない
- 跨言語呼び出しのインフラがない
- 自動的なバインディング生成ツールがない

### 設計哲学

YaoXiang は**「コアは簡潔に、複雑さはライブラリに下ろす」**の原則を守る：

> **良い品味 (Good Taste)**: 言語の責任は原子的な能力を提供することであり、全てを備えた機能集を提供することではない。複雑さはライブラリで解決すべきであり、コンパイラに蓄積すべきではない。

したがって、本案は：

- ✅ **構文変更ゼロ** — 完全な後方互換性、FFI エントリーポイントは `native("symbol")` のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで拡張
- ✅ **ツールチェーン自動化** — バインディングは `yx-bindgen` が自動生成し、手作業でのメンテナンスではない
- ✅ **漸進的強化** — 開発者は必要に応じて機能を取り込む

## 提案

### 1. コア FFI ライブラリの強化

`std.ffi` モジュールを拡張する。注意点：外部関数への呼び出しは全て引き続き `native("symbol")` 宣言経由であり、`std.ffi` は補助能力のみを提供する。

#### 1.1 動的ライブラリ読み込み

```yaoxiang
import ffi

# 動的ライブラリ (.so/.dll/.dylib) を読み込む
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、native で使用可能なシンボル名として返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols` はシンボル名を FfiRegistry の既知テーブルに登録する。その後ユーザーは引き続き `native` 宣言で使用する：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文はなく、`try_call` ラッパーもない。

#### 1.2 ライブラリ管理

```yaoxiang
# 読み込み済みのライブラリを一覧表示
loaded = ffi.loaded_libraries()

# ライブラリのアンロード
ffi.unload_library(lib)

# ライブラリバージョンのチェック
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol 構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

跨言語呼び出し規約と型変換は、実行時に汎用ラッパーでは処理せず、`yx-bindgen` がコンパイル時に生成する。

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

### 3. 多言語バインディング：ツールチェーン案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という幻想を諦める。代わりに公式ツールチェーンが自動生成する。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                                │
│                                               │
│  // ユーザーは native 宣言のみ書く                       │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時                   | 実行時
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (C ヘッダー → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディング生成器 (`yx-bindgen`)

`yx-bindgen` は独立した CLI ツールで、C ヘッダーファイルから YaoXiang FFI バインディングコードを生成する：

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
- 構造体レイアウトの整合（自動 `#[repr(C)]` 等価物）
- コールバック署名の変換

#### 3.3 公式メンテナンスのバインディングパッケージ

YaoXiang コアチームは全言語の汎用バインディングライブラリのメンテナンスを約束しないが、公式の `libc` バインディングパッケージ（POSIX + Windows API サブセット）を FFI 最佳事例と基础能力として提供する。

他の言語やライブラリのバインディング：

- `yx-bindgen` で自行生成
- YaoXiang パッケージとして公開可能（`libsqlite3`、`libcurl`、`libsdl2` など）
- コアチームはメンテナンスに責任を持たないが、パッケージの公開とバージョン管理メカニズムを提供する

### 4. 型変換層

#### 4.1 コンパイル時型マッピング

型変換は実行時ラッパー経由ではなく、`yx-bindgen` 生成時に静的に決定される：

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

FFI 境界を跨ぐすべてのメモリ割り当ては、以下の2つの質問に明確に答える必要がある：

1. **誰が割り当てた？** （C 側 `malloc` / YaoXiang 側ランタイム）
2. **誰が解放する？** （C 側 `free` / YaoXiang 側ランタイム）

`yx-bindgen` 生成時に、一般的なパターンに注釈を追加する：

```yaoxiang
# C 側で割り当て、呼び出し側が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し側がポインタを割り当て
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

実行時は FFI 境界を跨ぐポインタ参照に対して自動メモリ管理を行わない——所有権は呼び出し側に明確に帰属する。

#### 5.2 文字列処理

C 関数が返す `char*` は、YaoXiang `String` に変換される際に即座にコピーされる。原ポインタの所有権は C 関数决定的（ 注釈で宣言）であり、自动的には解放されない。

### 6. セキュリティ考量

#### 6.1 並行安全性

FFI 関数呼び出しは**デフォルトでは DAG スケジューリングに参加せず**、ブロッキング操作とみなす。reentrant であることが確認された C 関数には `@concurrent` マークを付与できる：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準 C ライブラリ関数に対して可能な限り reentrancy 情報をマークする（`strtok` の `_r` 変種など）。

**非同期呼び出し元への要件：** FFI 関数を呼び出す前に、呼び出し元は対象関数が reentrant であることを確認する必要がある。実行時は自動検出を行わない——これは静的解決不可能な問題であるため。

#### 6.2 エラー隔離

- FFI 呼び出しエラーは `Result` 型で伝播（関数が必要な Result 返り型を宣言している場合）
- タイムアウトメカニズムで外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry 層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全性

- ポインタ引数には YaoXiang 側で `unsafe` マークが必要
- FFI 境界を跨ぐポインタライフタイムは呼び出し元が保証

### 7. コンパイラ変更

**構文変更ゼロ** — `native("symbol")` 宣言のみ必要であり、現在のコンパイラ実装で既にサポート済み。

インタプリタ/ラン타임に追加する内容：

- 動的ライブラリ読み込み命令（`DynamicLibrary` の FFI バインディング）
- タイムアウトメカニズム

### 8. 採用されない機能

以下の機能は審査の結果として明確に除外され、RFC に含めない：

- **`ffi.try_call`**: 冗余であり、既存の `native` + `Result` 返り型で十分
- **`ffi.verify_signature`**: 実行時がコンパイラの仕事をしており、誤った抽象レベル
- **`ffi.async_call`**: reentrancy 契約モデルが明確になるまで待つ必要がある
- **コミュニティメンテナンスのバインディングテーブル**: 実行不可能であり、`yx-bindgen` ツールチェーン案に置き換え

## トレードオフ

###  長所

- ✅ **構文変更ゼロ** — FFI エントリーポイントは `native("symbol")` のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで漸進的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` がバインディング生成を自動処理
- ✅ **メモリ安全性** — 所有権モデルが明確で、自动解放による use-after-free なし
- ✅ **デバッグ可能** — エラーは OS ネイティブエラーコードを携带

### 短所

- ⚠️ 型安全性は C ヘッダーファイルの表現力に制限される（`void*` は静的識別不能）
- ⚠️ `yx-bindgen` は C 標準の進化に追従するために継続的メンテナンスが必要
- ⚠️ 非 C 言語（Python/JS/Java）のバインディングは各プロジェクトが自行処理し、统一案なし

## 実装戦略

### フェーズ 1：コアライブラリ (v0.7)

- [ ] `std.ffi` モジュールを拡張
- [ ] `DynamicLibrary` 構造を実装
- [ ] Linux/macOS サポート（`dlopen`/`dlsym`）
- [ ] Windows サポート（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウトメカニズムを追加
- [ ] ユニットテスト

### フェーズ 2：yx-bindgen (v0.8)

- [ ] C ヘッダーファイルパーサー実装（既存の Clang バインディングベースまたは自作パーサー）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言の生成
- [ ] 構造体レイアウトの生成
- [ ] 統合テスト：SQLite3、libcurl などの実際の C ライブラリでバインディング生成

### フェーズ 3：エコシステム基盤 (v0.9)

- [ ] 公式 `libc` バインディングパッケージ公開（POSIX + Windows API サブセット）
- [ ] バインディングパッケージ公開規範の制定
- [ ] ドキュメント：FFI 最佳事例、メモリ所有権、並行安全契約

## 他の RFC との関係

- **RFC-001**: FFI 呼び出しは外部関数として、デフォルトで `@block`（DAG スケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI 呼び出しは独立したタスク
- **RFC-020**: 動的モジュールと FFI 統合設計

## 未解決の問題

- [ ] `yx-bindgen` はビルドシステム（`yaoxiang build`）に統合する必要があるか？
- [ ] WASM プラットフォームの FFI サポートはどのように設計するか？（WASM のインポート機構は dlopen とは全く異なる）
- [ ] C++ name mangling を処理する `cxx-bindgen` を提供する必要があるか？（任意、v1.0 以降で検討）

---

## 付録 A：設計決定記録

| 決定 | 決定内容 | 理由 | 日付 | 記録者 |
|------|------|------|------|--------|
| FFI エントリーポイント統一化 | `native("symbol")` のみ残存 | API 分裂の回避 | 2026-05-29 | 晨煦 |
| `try_call` の除外 | 未実装 | 冗余、既存の Result 型で十分 | 2026-05-29 | 晨煦 |
| `verify_signature` の除外 | 未実装 | 実行時がコンパイラの仕事をしており | 2026-05-29 | 晨煦 |
| コミュニティバインディング → ツールチェーン | `yx-bindgen` 自動生成 | 実行不可能な幻想 | 2026-05-29 | 晨煦 |
| OS エラーコード | `FfiError` は必ず `os_error` を持つ | デバッグ不能な API は無用 | 2026-05-29 | 晨煦 |
| 構文変更ゼロ | ライブラリ実装に依存 | コア簡潔の原則 | 2026-03-14 | 晨煦 |
| 動的ライブラリ読み込み | dlopen/dlsym を使用 | 標準 OS インターフェース | 2026-03-14 | 晨煦 |
| エラー処理 | Result 型を使用 | 一貫性 | 2026-03-14 | 晨煦 |

## 付録 B：サンプルコード

### 完全例：C ライブラリの使用

```yaoxiang
# C 数学ライブラリを読み込む
libm = ffi.load_library("libm.so")

# C シンボルを実行時テーブルに登録（yx-bindgen がコンパイル時に実行）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native 宣言で使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗する可能性のある C 関数呼び出しでは Result を使用
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
- [FFI 実装計画](../reference/plan/completed/FFI.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)