# RFC-021: ライブラリ駆動型FFI拡張と異言語間呼び出しサポート

> **状態**: 草案
> **著者**: 晨煦
> **作成日**: 2026-03-14
> **最終更新**: 2026-05-29

> **参照**:
> - [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
> - [FFI実装計画](../reference/plan/completed/FFI.md)

## 摘要

本文書は**ライブラリ駆動型**のFFI（外部関数インターフェース）拡張案を提案する。FFIの唯一のエントリーポイントは`native("symbol")`宣言と`FfiRegistry`ランタイムレジストリであり、コアには第二のメカニズムを導入しない。在此基础上通过标准库提供动态库加载、跨语言调用绑定等能力。具体语言的调用绑定（如 C、Python、JavaScript）由官方工具链自动生成或由各项目按需编写。

## 動機

### 既存実装の不足

現在のFFI実装は以下の能力を備えている：

- `native("symbol")`構文による外部関数宣言
- `FfiRegistry`関数レジストリ

しかし機能が比較的単一である：

- 動的ライブラリ加载支持の欠如
- 異言語呼び出しのインフラストラクチャがない
- 自動的なバインディング生成ツール缺失

### 設計哲学

YaoXiangは**「コアはシンプルに、複雑さはライブラリに委ねる」**の原則に従う：

> **良い品味 (Good Taste)**: 言語の責任は原子的な能力を提供することであり、全てを揃えた機能セットを提供することではない。複雑さはライブラリで解決すべきであり、コンパイラに堆積させるべきではない。

因此、本案：

- ✅ **ゼロ構文変更** — 完全な後方互換性、FFIエントリーポイントは`native("symbol")`のみ
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで拡張
- ✅ **ツールチェーン自動化** — バインディングは`yx-bindgen`で自動生成、手作業ではない
- ✅ **漸進的強化** — 開発者は必要に応じて機能を導入

## 提案

### 1. コアFFIライブラリ強化

`std.ffi`モジュールを拡張する。注意：外部関数への呼び出しはすべて相変わらず`native("symbol")`宣言経由で行われ、`std.ffi`は補助能力のみを提供する。

#### 1.1 動的ライブラリ読込み

```yaoxiang
import ffi

# 動的ライブラリを読込む (.so/.dll/.dylib)
lib = ffi.load_library("./libmyext.so")

# ライブラリから関数シンボルを取得し、nativeで使用可能なシンボル名を返す
ffi.register_library_symbols(lib, [
    "my_function",
    "another_func",
])
```

`load_library`は`DynamicLibrary`ハンドルを返し、`register_library_symbols`はシンボル名をFfiRegistryの既知テーブルに登録する。その後でもっぱら`native`宣言を使用して：

```yaoxiang
my_func: (a: Int, b: Int) -> Int = native("my_function")
```

第二の呼び出し構文も`try_call`ラッパーもない。

#### 1.2 ライブラリ管理

```yaoxiang
# 読込み済みライブラリのリスト
loaded = ffi.loaded_libraries()

# ライブラリのアンロード
ffi.unload_library(lib)

# ライブラリバージョン照合
ffi.check_version(lib, "1.0.0")
```

#### 1.3 シンボル解決

```yaoxiang
# 名前でシンボルを検索（Symbol構造体を返す）
sin_sym = ffi.dlsym("libm.so", "sin")
```

異言語呼び出し規約と型変換はランタイムで汎用ラッパー経由で処理するのではなく、`yx-bindgen`がコンパイル時に生成する。

### 2. 動的ライブラリ読込み実装

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

`OsError`はプラットフォームネイティブのエラーコード（Linuxの`dlerror()`、Windowsの`GetLastError()`）を携带し、デバッグ可能性を保証する。

### 3. 多言語バインディング：ツールチェーン案

「コミュニティの各言語メンテナーがバインディングライブラリを書く」という幻想を捨てる。代わりに公式ツールチェーンで自動生成する。

#### 3.1 アーキテクチャ設計

```
┌───────────────────────────────────────────────┐
│  YaoXiang コード                                │
│                                               │
│  // ユーザーはnative宣言のみを書く               │
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

`yx-bindgen`は独立したCLIツールで、CヘッダーファイルからYaoXiang FFIバインディングコードを生成する：

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

`yx-bindgen`は公式にメンテナンスされ、以下を保証する：

- 完全な型マッピング（`int` → `Int`、`char*` → `*const u8`、`void*` → `*mut opaque`）
- 構造体レイアウトの整列（自動`#[repr(C)]`相当）
- コールバック署名の変換

#### 3.3 公式メンテナンスバインディングパッケージ

YaoXiangコアチームは全言語の汎用バインディングライブラリのメンテナンスを約束しないが、FFI最佳実践例および基本能力として、公式の`libc`バインディングパッケージ（POSIX + Windows APIサブセット）を提供する。

その他言語とライブラリのバインディング：

- `yx-bindgen`で自行生成
- YaoXiangパッケージとして公開可能（`libsqlite3`、`libcurl`、`libsdl2`など）
- コアチームはメンテナンスに責任を持たないが、パッケージの公開とバージョン管理メカニズムを提供する

### 4. 型変換層

#### 4.1 コンパイル時型マッピング

型変換はランタイムラッパー経由ではなく、`yx-bindgen`生成時に静的に決定する：

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

FFI境界を越えるメモリ割り当ては、以下の二つの問いに明確に答えなければならない：

1. **誰が確保したか？** （C側`malloc` / YaoXiang側ランタイム）
2. **誰が解放するか？** （C側`free` / YaoXiang側ランタイム）

`yx-bindgen`生成時、共通パターンには注釈を追加する：

```yaoxiang
# Cが確保、呼び出し元が解放
sqlite3_exec: (...) -> Int
    = native("sqlite3_exec")
    # memory: C-allocated, caller must free errmsg via sqlite3_free

# 呼び出し元がポインタを確保
read: (fd: Int, buf: *mut u8, count: Int) -> Int
    = native("read")
    # memory: caller-allocated buf
```

ランタイムはFFI境界を越えるポインタ参照の自動メモリ管理を行わない — 所有権は明確に呼び出し元にある。

#### 5.2 文字列処理

C関数が返す`char*`は、YaoXiang`String`に変換される際に直ちにコピーされる。原ポインタの所有権はC関数次第（註釈で宣言）であり、自動的に解放されない。

### 6. 安全性考量

#### 6.1 並行安全

FFI関数呼び出しは**デフォルトでDAGスケジューリングに参加せず**、ブロッキング操作と見なす。再入可能と確認されたC関数には`@concurrent`マークを付与できる：

```yaoxiang
# 純粋関数、グローバル状態なし、並行可能
sin_safe: (x: Float) -> Float = native("sin")
    # reentrant: true

# グローバル状態あり、並行不可
strtok: (s: *const u8, delim: *const u8) -> *const u8 = native("strtok")
    # reentrant: false
```

`yx-bindgen`は標準Cライブラリ関数に可能な限りの再入可能性情報（`strtok`の`_r`バリアントなど）を标注する。

**非同期呼び出し元への要件：** FFI関数を呼び出す前に、呼び出し元は対象関数が再入可能であることを確認しなければならない。ランタイムは自動検出を行わない — これは静的に解決できない問題である。

#### 6.2 エラー隔離

- FFI呼び出しエラーは`Result`型経由で伝播（関数がResult戻り値型を宣言している場合）
- タイムアウトメカニズムで外部関数のデッドロックを防止

```yaoxiang
# タイムアウト付き呼び出し（FfiRegistry層で実装）
result = ffi.call_with_timeout("blocking_func", 5000)  # 5秒タイムアウト
```

#### 6.3 ポインタ安全

- ポインタ引数にはYaoXiang側で`unsafe`マークが必要
- FFI境界を越えるポインタのライフタイムは呼び出し元が保証

### 7. コンパイラ変更

**ゼロ構文変更** — `native("symbol")`宣言のみが必要であり、現在のコンパイラ実装に既に存在している。

解釈実行系/ランタイムに追加：

- 動的ライブラリ読込み命令（`DynamicLibrary`のFFIバインディング）
- タイムアウトメカニズム

### 8. 否決された機能

以下の機能は審査を経て明確に除外され、本RFCに含めない：

- **`ffi.try_call`**: 冗長、既存の`native` + `Result`戻り値型で十分
- **`ffi.verify_signature`**: ランタイムでコンパイラの仕事をしており、誤った抽象レベル
- **`ffi.async_call`**: 再入可能性契約モデルが明確になってからの検討が必要
- **コミュニティメンテナンスバインディング表**: 実行不可能、`yx-bindgen`ツールチェーン案に改めた

## 权衡

### 优点

- ✅ **ゼロ構文変更** — FFIエントリーポイントは`native("symbol")`のみ、完全な後方互換性
- ✅ **ライブラリ即ち言語** — 機能は標準ライブラリで漸進的に導入
- ✅ **ツールチェーン駆動** — `yx-bindgen`がバインディング生成を自動処理
- ✅ **メモリ安全** — 所有権モデルが明確、自動回収によるuse-after-freeなし
- ✅ **デバッグ可能** — エラーはOSネイティブエラーコード携带

### 缺点

- ⚠️ 型安全はCヘッダーファイルの表現力に制限される（`void*`は静的識別不可）
- ⚠️ `yx-bindgen`はC標準の進化に追従するために継続的なメンテナンスが必要
- ⚠️ C以外（Python/JS/Java）のバインディングは各プロジェクトが自行処理、统一案なし

## 実装戦略

### フェーズ1：コアライブラリ (v0.7)

- [ ] `std.ffi`モジュールを拡張
- [ ] `DynamicLibrary`構造体を実装
- [ ] Linux/macOSサポート（`dlopen`/`dlsym`）
- [ ] Windowsサポート（`LoadLibrary`/`GetProcAddress`）
- [ ] ランタイムにタイムアウトメカニズムを追加
- [ ] ユニットテスト

### フェーズ2：yx-bindgen (v0.8)

- [ ] Cヘッダーファイルパーサーを実装（既存のClangバインディングまたは手書きパーサー）
- [ ] 型マッピングシステムを構築
- [ ] `native("symbol")`宣言を生成
- [ ] 構造体レイアウトを生成
- [ ] 統合テスト：SQLite3、libcurlなど実在Cライブラリのバインディング生成

### フェーズ3：エコシステム基盤 (v0.9)

- [ ] 公式`libc`バインディングパッケージを公開（POSIX + Windows APIサブセット）
- [ ] バインディングパッケージ公開規範を制定
- [ ] 文書化：FFI最佳実践、メモリ所有権、並行安全契約

## 他のRFCとの関係

- **RFC-001**: FFI呼び出しは外部関数として、デフォルト`@block`（DAGスケジューリングに参加しない）
- **RFC-008**: スケジューラ分離設計、FFI呼び出しは独立タスク
- **RFC-020**: 動的モジュールとFFI統合設計

## 開放問題

- [ ] `yx-bindgen`はビルドシステム（`yaoxiang build`）に統合する必要があるか？
- [ ] WASMプラットフォームのFFIサポートはどう設計するか？（WASMのインポート機構はdlopenと全く異なる）
- [ ] C++名前マングリングを処理する`cxx-bindgen`は必要か？（任意、v1.0以降で検討）

---

## 付録A：設計決定記録

| 決定 | 決定内容 | 理由 | 日付 | 記録者 |
|------|------|------|------|--------|
| FFIエントリーポイントの一元化 | `native("symbol")`のみ保持 | API分裂の回避 | 2026-05-29 | 晨煦 |
| `try_call`の除外 | 実装しない | 冗長、Result型が既存 | 2026-05-29 | 晨煦 |
| `verify_signature`の除外 | 実装しない | ランタイムでコンパイラの仕事をしており | 2026-05-29 | 晨煦 |
| コミュニティバインディング → ツールチェーン | `yx-bindgen`自動生成 | 実行不可能な幻想 | 2026-05-29 | 晨煦 |
| OSエラーコード | `FfiError`は必ず`os_error`を携带 | デバッグ不可能なAPIは無用 | 2026-05-29 | 晨煦 |
| ゼロ構文変更 | ライブラリ実装に依存 | コアシンプル原則 | 2026-03-14 | 晨煦 |
| 動的ライブラリ読込み | dlopen/dlsymを使用 | 標準OSインターフェース | 2026-03-14 | 晨煦 |
| エラー処理 | Result型を使用 | 一貫性 | 2026-03-14 | 晨煦 |

## 付録B：サンプルコード

### 完全例：Cライブラリを使用

```yaoxiang
# C数学ライブラリを読込む
libm = ffi.load_library("libm.so")

# Cシンボルをランタイムテーブルに登録（yx-bindgenがコンパイル時にこれを行う）
ffi.register_library_symbols(libm, ["sin", "cos", "sqrt"])

# native宣言で使用
sin_f: (x: Float) -> Float = native("sin")
cos_f: (x: Float) -> Float = native("cos")

# 直接呼び出し
result = sin_f(3.14159 / 2)

# 失敗可能性のあるC関数を呼び出す時はResultを使用
file_open: (path: *const u8, mode: *const u8) -> Result(*mut opaque, Int)
    = native("fopen")
```

### yx-bindgenを使用

```bash
# 全宣言を自動生成、手書き不要
yx-bindgen --header /usr/include/math.h --output math_bindings.yx

# YaoXiangでインポート
import "math_bindings.yx"
# sin_f / cos_fなどは自動的にnative("sin") / native("cos")として宣言済み
```

---

## 参考文献

- [RFC-001: 並作モデルとエラー処理システム](./001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 並行モデルとスケジューラ分離設計](./008-runtime-concurrency-model.md)
- [FFI実装計画](../reference/plan/completed/FFI.md)
- [Python ctypes ドキュメント](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading crate](https://docs.rs/libloading/latest/libloading/)