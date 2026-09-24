---
title: 'RFC-021: ライブラリ駆動 FFI 拡張と言語間呼び出しサポート'
status: '廃止'
author: '晨煦'
created: '2026-03-14'
updated: '2026-06-05（廃止）'
---

# RFC-021: ライブラリ駆動 FFI 拡張と言語間呼び出しサポート

> **⚠️ 廃止**：本文書は廃止され、内容は
> [RFC-026: FFI コアメカニズム](../accepted/026-ffi-core-mechanism.md) にマージされました。

> **参考**:
>
> - [RFC-001: spawn モデルとエラーハンドリングシステム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行性モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
> - [FFI 実装計画](../reference/plan/completed/FFI.md)

## 概要

本文書では、**ライブラリ駆動**の FFI（外部関数インターフェース）拡張方案を提案する。FFI の唯一の入口は
`native("symbol")` 宣言と `FfiRegistry`
ランタイムレジストリであり、コアに第二のメカニズムを導入しない。その上に標準ライブラリを通じて動的ライブラリのロードや言語間呼び出しバインディングなどの機能を提供する。特定の言語の呼び出しバインディング（C、Python、JavaScript など）は、公式ツールチェーンによって自動生成されるか、各プロジェクトが必要に応じて記述する。

## 動機

### 既存実装の不足

現在の FFI 実装には以下の機能がある：

- `native("symbol")` 構文による外部関数の宣言
- `FfiRegistry` 関数レジストリ

しかし、機能は比較的単一である：

- 動的ライブラリロードのサポートがない
- 言語間呼び出しのためのインフラがない
- 自動化されたバインディング生成ツールがない

### 設計哲学

YaoXiang は **「コアはシンプルに、複雑性はライブラリに下沉させる」** という原則に従う：

> **良品味 (Good
> Taste)**: 言語の責務は原子的な能力を提供することであり、包括的な機能セットではない。複雑性はコンパイラに蓄積するのではなく、ライブラリを通じて解決すべきである。

したがって、本方案では：

- ✅ **ゼロ構文変更** — 完全な後方互換性、FFI 入口は `native("symbol")` のみ
- ✅ **ライブラリこそが言語** — 機能は標準ライブラリを通じて拡張
- ✅ **ツールチェーン自動化** — バインディングは `yx-bindgen` で自動生成、手動保守ではない
- ✅ **段階的強化** — 開発者が必要に応じて機能を導入

## 提案

### 1. コア FFI ライブラリの強化

`std.ffi` モジュールを拡張する。注意：すべての外部関数の呼び出しは引き続き `native("symbol")`
宣言を介して行われ、`std.ffi` は補助的な機能のみを提供する。

#### 1.1 動的ライブラリロード

```yaoxiang
import ffi

# 動的ライブラリをロードする (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、native で使用可能な symbol 名として返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library` は `DynamicLibrary` ハンドルを返し、`register_library_symbols`
はシンボル名を FfiRegistry の既知の表に登録する。その後、ユーザは引き続き `native`
宣言を介して使用する：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文は存在せず、`try_call` ラッパーも存在しない。

#### 1.2 ライブラリ管理

```yaoxiang
# ロード済みライブラリの一覧を取得する
loaded = ffi.loaded_libraries()

# ライブラリをアンロードする
ffi.unload_library(lib)

# ライブラリのバージョンチェック
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索する（Symbol 構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

言語間呼び出し規約や型変換は、ランタイムで汎用ラッパーを介して処理するのではなく、`yx-bindgen`
によってコンパイル時に生成される。

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

`OsError` はプラットフォームネイティブのエラーコード（Linux の `dlerror()`、Windows の
`GetLastError()`）を運び、デバッグ可能性を保証する。

### 3. 多言語バインディング：ツールチェーン方案

「コミュニティの各言語メンテナがバインディングライブラリを保守する」という幻想は捨て、公式ツールチェーンによる自動生成に切り替える。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                              │
│                                               │
│  // ユーザは native 宣言のみを記述する         │
│  my_func: (a: Int) -> Int = native("my_func") │
└───────────────────────────────────────────────┘
         ↑                          ↑
         |  コンパイル時             | ランタイム
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

`yx-bindgen` は公式に保守され、以下を保証する：

- 型マッピングの完全性（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体レイアウトのアライメント（自動的に `#[repr(C)]` 等価物を付与）
- callback シグネチャ変換

#### 3.3 公式保守バインディングパッケージ

YaoXiang コアチームはすべての言語の汎用バインディングライブラリの保守を約束はしないが、公式の `libc`
バインディングパッケージ（POSIX + Windows
API サブセット）を提供し、FFI ベストプラクティスの例および基礎能力とする。

その他の言語やライブラリのバインディング：

- `yx-bindgen` を使用して自行で生成
- YaoXiang パッケージとして公開可能（例：`libsqlite3`、`libcurl`、`libsdl2`）
- コアチームは保守を担当しないが、パッケージの公開とバージョン管理の仕組みを提供

### 4. 型変換層

#### 4.1 コンパイル時の型マッピング

型変換はランタイムラッパーを介さず、`yx-bindgen` の生成時に静的に決定される：

| C 型         | YaoXiang 型       | 変換方式                     |
| ------------ | ----------------- | ---------------------------- |
| `int`        | `Int`             | 値渡し                       |
| `char*`      | `*const u8`       | ポインタ渡し                 |
| `void*`      | `*mut opaque`     | 不透明ポインタ               |
| `struct T`   | `extern struct T` | メモリレイアウト一致         |
| `int*`       | `*mut Int`        | ポインタ渡し（可変）         |
| `const int*` | `*const Int`      | ポインタ渡し（読み取り専用） |

#### 4.2 手動変換（標準ライブラリ補助）

```yaoxiang
# 明示的な変換
raw_ptr = ffi.to_pointer(my_bytes)
c_string = ffi.to_c_string(my_string)
```

### 5. メモリ所有権モデル

#### 5.1 基本原則

FFI 境界を跨ぐすべてのメモリ割り当ては、以下の 2 つの質問に明確に答えなければならない：

1. **誰が割り当てるか？**（C 側 `malloc` / YaoXiang 側ランタイム）
2. **誰が解放するか？**（C 側 `free` / YaoXiang 側ランタイム）

`yx-bindgen` の生成時に、一般的なパターンに対して注釈を追加する：

```yaoxiang
# C 側で割り当て、呼び出し側が解放する
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し側がポインタを割り当てる
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

ランタイムは FFI 境界を跨ぐポインタ参照に対して自動メモリ管理を行わない — 所有権は明確に呼び出し側に帰属する。

#### 5.2 文字列処理

C 関数が返す `char*` は、YaoXiang の `String`
に変換される際に直ちにコピーされる。元ポインタの所有権は C 関数によって決定され（注釈で宣言される）、自動的には解放されない。

### 6. 安全性に関する考慮

#### 6.1 並行性安全性

FFI 関数呼び出しは**デフォルトでは DAG スケジューリングに参加せず**、ブロッキング操作として扱われる。reentrant であることが確認された C 関数は
`@concurrent` でマーク可能：

```yaoxiang
# 純粋関数でグローバル状態を持たないため並行呼び出し可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態を持つため並行呼び出し不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen` は標準 C ライブラリ関数に対して可能な限り reentrancy 情報を注釈する（`strtok` の `_r`
バリアントなど）。

**非同期呼び出し側への要件：**
FFI 関数を呼び出す前に、呼び出し側は対象関数が reentrant であることを保証しなければならない。ランタイムは自動検出を行わない — これは静的に解決不可能な問題である。

#### 6.2 エラー隔離

- FFI 呼び出しエラーは `Result` 型で伝播する（関数が Result 戻り型を宣言している場合）
- タイムアウト機構により外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry レイヤで実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全性

- ポインタ引数には YaoXiang 側の `unsafe` マークが必要
- FFI 境界を跨ぐポインタのライフタイムは呼び出し側が保証する

### 7. コンパイラの変更点

**ゼロ構文変更** — `native("symbol")` 宣言のみが必要で、すでに現在のコンパイラに実装されている。

インタプリタ/ランタイムに追加するもの：

- 動的ライブラリロード命令（`DynamicLibrary` の FFI バインディング）
- タイムアウト機構

### 8. 採用されない機能

以下の機能は審査の結果、明示的に除外され、RFC に含めない：

- **`ffi.try_call`**: 冗長であり、すでに `native` + `Result` 戻り型がある
- **`ffi.verify_signature`**: ランタイムでコンパイラの作業を行うのは、誤った抽象化レイヤー
- **`ffi.async_call`**: reentrancy 契約モデルが明確になるまで保留
- **コミュニティ保守バインディング表**: 実行不可能なため、`yx-bindgen` ツールチェーン方案に変更

## トレードオフ

### 利点

- ✅ **ゼロ構文変更** — FFI 入口は `native("symbol")` のみ、完全な後方互換性
- ✅ **ライブラリこそが言語** — 機能は標準ライブラリを通じて段階的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen` がバインディング生成を自動化
- ✅ **メモリ安全性** — 所有権モデルが明確で、自動回収による use-after-free がない
- ✅ **デバッグ可能** — エラーに OS ネイティブのエラーコードが含まれる

### 欠点

- ⚠️ 型安全性は C ヘッダファイルの表現力に制限される（`void*` は静的に区別不可）
- ⚠️ `yx-bindgen` は C 標準の進化に追随するために継続的な保守が必要
- ⚠️
  C 以外の言語（Python/JS/Java）のバインディングは各プロジェクトが自行で処理する必要があり、統一方案がない

## 実装戦略

### フェーズ 1：コアライブラリ (v0.7)

- [ ] `std.ffi` モジュールを拡張
- [ ] `DynamicLibrary` 構造を実装
- [ ] Linux/macOS サポート（`dlopen`/`dlsym`）
- [ ] Windows サポート（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウト機構を追加
- [ ] ユニットテスト

### フェーズ 2：yx-bindgen (v0.8)

- [ ] C ヘッダファイルパーサーを実装（既存の Clang バインディングまたは手書きパーサーをベース）
- [ ] 型マッピングシステム
- [ ] `native("symbol")` 宣言の生成
- [ ] 構造体レイアウトの生成
- [ ] 統合テスト：SQLite3、libcurl などの実 C ライブラリに対するバインディング生成

### フェーズ 3：エコシステム基盤 (v0.9)

- [ ] 公式 `libc` バインディングパッケージを公開（POSIX + Windows API サブセット）
- [ ] バインディングパッケージ公開規約を策定
- [ ] ドキュメント：FFI ベストプラクティス、メモリ所有権、並行性安全性契約

## 他の RFC との関係

- **RFC-001**: FFI 呼び出しは外部関数として、デフォルトでは
  `@block`（DAG スケジューリングに参加しない）
- **RFC-008**: スケジューラの疎結合設計、FFI 呼び出しは独立したタスクとして
- **RFC-020**:
  FFI ノードの DAG におけるスケジューリングセマンティクス、Phi ノード、ループ展開など、スケジューリングレイヤの詳細設計

## 未解決問題

- [ ] `yx-bindgen` をビルドシステム（`yaoxiang build`）に統合すべきか？
- [ ] WASM プラットフォームの FFI サポートはどのように設計すべきか？（WASM のインポート機構は dlopen と根本的に異なる）
- [ ] C++ name mangling を処理する `cxx-bindgen` を提供すべきか？（オプション、v1.0 以降で検討）

---

## 付録 A：設計決定記録

| 決定                              | 決定内容                              | 理由                         | 日付       | 記録者 |
| --------------------------------- | ------------------------------------- | ---------------------------- | ---------- | ------ |
| FFI 入口の一意化                  | `native("symbol")` のみを保持         | API の分裂を避ける           | 2026-05-29 | 晨煦   |
| `try_call` の除外                 | 実装しない                            | 冗長、すでに Result 型がある | 2026-05-29 | 晨煦   |
| `verify_signature` の除外         | 実装しない                            | ランタイムでコンパイラの作業 | 2026-05-29 | 晨煦   |
| コミュニティ保守 → ツールチェーン | `yx-bindgen` による自動生成           | 実行不可能な幻想             | 2026-05-29 | 晨煦   |
| OS エラーコード                   | `FfiError` に必ず `os_error` を含める | デバッグ不能な API は無用    | 2026-05-29 | 晨煦   |
| ゼロ構文変更                      | ライブラリ実装に依存                  | コアはシンプルに原則         | 2026-03-14 | 晨煦   |
| 動的ライブラリロード              | dlopen/dlsym を使用                   | 標準 OS インターフェース     | 2026-03-14 | 晨煦   |
| エラーハンドリング                | Result 型を使用                       | 一貫性                       | 2026-03-14 | 晨煦   |

## 付録 B：サンプルコード

### 完全なサンプル：C ライブラリの使用

```yaoxiang
# C 数学ライブラリをロード
libm = ffi.load_library("libm.so")

# C シンボルをランタイム表に登録する（yx-bindgen はコンパイル時にこれを行う）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native 宣言を介して使用する
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗する可能性のある C 関数を呼び出す場合は Result を使用
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### yx-bindgen の使用

```bash
# すべての宣言を自動生成、手書き不要
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# YaoXiang でインポート
import "math_bindings.yx"
# sin_f / cos_f などは native("sin") / native("cos") として自動宣言される
```

---

## 参考文献

- [RFC-001: spawn モデルとエラーハンドリングシステム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行性モデルとスケジューラの疎結合設計](../accepted/008-runtime-concurrency-model.md)
- [FFI 実装計画](../reference/plan/completed/FFI.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)
