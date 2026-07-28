---
title: 'RFC-021: ライブラリ駆動型 FFI 拡張と跨言語呼び出しサポート'
status: '廃止'
author: '晨煦'
created: '2026-03-14'
updated: '2026-06-05（廃止）'
---

# RFC-021: ライブラリ駆動型 FFI 拡張と跨言語呼び出しサポート

> **⚠️ 廃止**: 本ドキュメントは廃止され、内容は [RFC-026: FFI コアメカニズム](./026-ffi-core-mechanism.md) に統合されました。

> **参考**:
>
> - [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並発モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
> - [FFI 実装計画](../reference/plan/completed/FFI.md)

## 摘要

本ドキュメントでは、**ライブラリ駆動型**の FFI（外部関数インターフェース）拡張案を提案します。FFI の唯一のエントリーポイントは `native("symbol")` 宣言 +
`FfiRegistry` ランタイムレジストリとし、コアには第二のメカニズムを導入しません。在此基础上通过标准库提供动态库加载、跨语言调用绑定等能力。具体语言的调用绑定（如 C、Python、JavaScript）由官方工具链自动生成或由各项目按需编写。

## 動機

### 既存実装の不足

現在の FFI 実装には以下の能力があります：

- `native("symbol")` 構文による外部関数宣言
- `FfiRegistry` 関数レジストリ

しかし、機能的には限定的です：

- 動的ライブラリのロードサポートが欠けている
- 跨言語呼び出しのインフラストラクチャがない
- 自動化されたバインディング生成ツールがない

### 設計哲学

YaoXiang は **"コアはシンプルに、複雑性はライブラリに任せる"** の原則に従います：

> **良い品味 (Good
> Taste)**: 言語の責務は原子的な能力を提供することであり、全てを備えた機能集を提供することではありません。複雑性はライブラリで解決すべきであり、コンパイラに蓄積すべきではありません。

したがって、本方案では：

- ✅ **構文変更ゼロ** — 完全な後方互換性、FFI のエントリーポイントは `native("symbol")` のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで拡張
- ✅ **ツールチェーン自動化** — バインディングは `yx-bindgen` が自動生成、手作業ではない
- ✅ **漸進的強化** — 開発者は必要に応じて機能を導入

## 提案

### 1. コア FFI ライブラリの強化

`std.ffi` モジュールを拡張します。注意：外部関数への呼び出しはすべて引き続き `native("symbol")` 宣言を通じて行われ、`std.ffi` は補助能力のみを提供します。

#### 1.1 動的ライブラリのロード

```yaoxiang
import ffi

# 動的ライブラリ (.so/.dll/.dylib) をロード
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、FfiRegistry の既知テーブルに登録
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols` はシンボル名を FfiRegistry の既知テーブルに登録します。その後、ユーザーは引き続き `native` 宣言で使用します：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文も `try_call` ラッパーもありません。

#### 1.2 ライブラリ管理

```yaoxiang
# ロード済みのライブラリを一覧表示
loaded = ffi.loaded_libraries()

# ライブラリをアンロード
ffi.unload_library(lib)

# ライブラリバージョン検査
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol 構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

跨言語呼び出し規約と型変換は、运行时不通过通用包装器处理，而是由 `yx-bindgen` がコンパイル時に生成します。

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

`OsError` はプラットフォームネイティブのエラーコード（Linux の `dlerror()`、Windows の `GetLastError()`）を携带し、デバッグ可能性を確保します。

### 3. 多言語バインディング：ツールチェーン案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という幻想を諦めます。公式ツールチェーンによる自動生成に方針を変更します。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                                │
│                                               │
│  // ユーザーは native 宣言のみを書く             │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時                  | ランタイム
┌──────────────────┐   ┌────────────────────────┐
│  yx-bindgen       │   │  std.ffi + FfiRegistry  │
│  (C ヘッダー → .yx) │   │  - dlopen/dlsym         │
│                   │   │  - LoadLibrary/GetProc  │
└──────────────────┘   └────────────────────────┘
```

#### 3.2 バインディング生成器 (`yx-bindgen`)

`yx-bindgen` は独立した CLI ツールで、C ヘッダーファイルから YaoXiang FFI バインディングコードを生成します：

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
- 構造体レイアウトの整合（自動 `#[repr(C)]` 等価物）
- コールバック署名の変換

#### 3.3 公式メンテナンスのバインディングパッケージ

YaoXiang コアチームは全ての言語の汎用バインディングライブラリのメンテナンスを約束しませんが、公式の `libc` バインディングパッケージ（POSIX + Windows
API サブセット）を提供し、FFI のベストプラクティス例と基盤能力とします。

他の言語やライブラリのバインディング：

- `yx-bindgen` を使用して自行生成
- YaoXiang パッケージとしてリリース可能（例：`libsqlite3`、`libcurl`、`libsdl2`）
- コアチームはメンテナンスを担当しませんが、パブリッシュとバージョン管理の仕組みを提供

### 4. 型変換層

#### 4.1 コンパイル時型マッピング

型変換はランタイムラッパーを通さず、`yx-bindgen` 生成時に静的に決定されます：

| C 型          | YaoXiang 型       | 変換方式           |
| ------------- | ----------------- | ------------------ |
| `int`         | `Int`             | 直接値渡し         |
| `char*`       | `*const u8`       | ポインタ渡し       |
| `void*`       | `*mut opaque`     | 不透明ポインタ     |
| `struct T`    | `extern struct T` | メモリレイアウト整合 |
| `int*`        | `*mut Int`        | ポインタ渡し（可変） |
| `const int*`  | `*const Int`      | ポインタ渡し（読み取り専用） |

#### 4.2 手動変換（標準ライブラリ補助）

```yaoxiang
# 明示的変換
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. メモリ所有権モデル

#### 5.1 基本原則

跨 FFI 境界の各メモリ割り当てについて、以下の2つの問いに明確に答える必要があります：

1. **誰が割り当てた？** （C 側 `malloc` / YaoXiang 側ランタイム）
2. **誰が解放する？** （C 側 `free` / YaoXiang 側ランタイム）

`yx-bindgen` 生成時に、一般的なパターンに注釈を追加します：

```yaoxiang
# C が割り当て、呼び出し元が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し元がポインタを割り当て
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

ランタイムは跨 FFI 境界のポインタ参照に対して自動メモリ管理を行わず、所有権は呼び出し元に明確にあります。

#### 5.2 文字列処理

C 関数が返す `char*` は、YaoXiang `String` に変換される際に即座にコピーされます。元のポインタの所有権は C 関数決定し（注釈で宣言）、自動解放しません。

### 6. 安全性への考慮

#### 6.1 並発安全性

FFI 関数呼び出しは**デフォルトでは DAG スケジューリングに参加せず**、ブロッキング操作とみなします。再入可能であることが確認された C 関数には `!concurrent` マークを付けることができます：

```yaoxiang
# 純粋関数、グローバル状態なし、並発可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並発不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準 C ライブラリ関数について可能な限り再入可能性情報をマークします（`strtok` の `_r` バリアントなど）。

**非同期呼び出し元への要求：**
FFI 関数を呼び出す前に、呼び出し元は対象関数が再入可能であることを確認する必要があります。ランタイムは自動検出を行いません—これは静的に解決できない問題です。

#### 6.2 エラー隔離

- FFI 呼び出しエラーは `Result` 型で伝播（関数が Result 戻り値型を宣言している場合）
- タイムアウト機構により外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry 層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全

- ポインタ引数には YaoXiang 側で `unsafe` マークが必要
- 跨 FFI 境界のポインタライフタイムは呼び出し元が保証

### 7. コンパイラへの変更

**構文変更ゼロ** — `native("symbol")` 宣言のみで済み、現在のコンパイラ実装で対応済み。

インタプリタ/ランタイムに追加：

- 動的ライブラリロード命令（`DynamicLibrary` の FFI バインディング）
- タイムアウト機構

### 8. 採用されない機能

以下の機能は審査の結果、明確に除外され、RFC に含めません：

- **`ffi.try_call`**: 不要、`native` + `Result` 戻り値型ですでにある
- **`ffi.verify_signature`**: ランタイムでコンパイラの仕事をしており、誤った抽象レベル
- **`ffi.async_call`**: 再入可能性契約モデルが明確になってからの検討が必要
- **コミュニティメンテナンスのバインディングテーブル**: 実行不可能、`yx-bindgen` ツールチェーン案に変更

## 权衡

### 利点

- ✅ **構文変更ゼロ** — FFI エントリーポイントは `native("symbol")` のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで漸進的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` がバインディング生成を自動処理
- ✅ **メモリ安全** — 所有権モデルが明確、自動回收による use-after-free なし
- ✅ **デバッグ可能** — エラーは OS ネイティブエラーコード携带

### 欠点

- ⚠️ 型安全は C ヘッダーファイルの表現力に限られる（`void*` は静的識別不可）
- ⚠️ `yx-bindgen` は C 標準の進化に追従するために継続的なメンテナンスが必要
- ⚠️ 非 C 言語（Python/JS/Java）のバインディングは各プロジェクトで自行処理、统一案なし

## 実装戦略

### フェーズ 1: コアライブラリ (v0.7)

- [ ] `std.ffi` モジュールを拡張
- [ ] `DynamicLibrary` 構造体を実装
- [ ] Linux/macOS サポート（`dlopen`/`dlsym`）
- [ ] Windows サポート（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウト機構を追加
- [ ] ユニットテスト

### フェーズ 2: yx-bindgen (v0.8)

- [ ] C ヘッダーファイルパーサー実装（既存の Clang バインディングまたは手書きパーサー）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言を生成
- [ ] 構造体レイアウトを生成
- [ ] 統合テスト：SQLite3、libcurl などの実際の C ライブラリでバインディング生成

### フェーズ 3: エコシステム基盤 (v0.9)

- [ ] 公式 `libc` バインディングパッケージをリリース（POSIX + Windows API サブセット）
- [ ] バインディングパッケージ公開規程の制定
- [ ] ドキュメント：FFI ベストプラクティス、メモリ所有権、並発安全契約

## 他の RFC との関係

- **RFC-001**: FFI 呼び出しは外部関数として、デフォルトで `@block`（DAG スケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI 呼び出しは独立したタスク
- **RFC-020**: DAG 内での FFI ノードのスケジューリングセマンティクス、Phi ノード、循环展開などのスケジューリング層の設計

## 开放问题

- [ ] `yx-bindgen` はビルドシステム（`yaoxiang build`）に統合する必要がありますか？
- [ ] WASM プラットフォームの FFI サポートはどのように設計しますか？（WASM のインポート機構は dlopen 完全不同）
- [ ] C++ name mangling を処理する `cxx-bindgen` を提供する必要がありますか？（オプション、v1.0 以後に検討）

---

## 付録 A：設計意思決定記録

| 意思決定                  | 決定                         | 理由                      | 日付       | 記録者 |
| ------------------------- | ---------------------------- | ------------------------- | ---------- | ------ |
| FFI エントリーポイント統一 | `native("symbol")` のみ保持   | API 分裂の回避            | 2026-05-29 | 晨煦   |
| `try_call` の除外         | 実装しない                   | 不要、Result 型ですでにある | 2026-05-29 | 晨煦   |
| `verify_signature` の除外 | 実装しない                   | ランタイムでコンパイラの作業 | 2026-05-29 | 晨煦   |
| コミュニティバインディング → ツールチェーン | `yx-bindgen` 自動生成      | 実行不可能な幻想          | 2026-05-29 | 晨煦   |
| OS エラーコード           | `FfiError` は `os_error` 必須 | デバッグ不能な API は無意味 | 2026-05-29 | 晨煦   |
| 構文変更ゼロ              | ライブラリ実装に依存         | コアシンプル原則          | 2026-03-14 | 晨煦   |
| 動的ライブラリロード      | dlopen/dlsym を使用          | 標準 OS インターフェース  | 2026-03-14 | 晨煦   |
| エラー処理                | Result 型を使用              | 一貫性                    | 2026-03-14 | 晨煦   |

## 付録 B：サンプルコード

### 完全例：C ライブラリの使用

```yaoxiang
# C 数学ライブラリをロード
libm = ffi.load_library("libm.so")

# C シンボルをランタイムテーブルに登録（yx-bindgen がコンパイル時に実行）
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
# sin_f / cos_f などは native("sin") / native("cos") として自動宣言済み
```

---

## 参考文献

- [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並発モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
- [FFI 実装計画](../reference/plan/completed/FFI.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)
