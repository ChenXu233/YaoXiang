---
title: "FFI 実装計画"
---

# FFI 実装計画

## 概要

FFI（Foreign Function Interface）メカニズムは、YaoXiang コードが Rust ネイティブ関数を呼び出すことを可能にし、言語ランタイムとシステム API を接続する架け橋です。本計画では、コンパイル時バインディングの FFI メカニズムを実装し、以下の機能をサポートします：

- 標準ライブラリ関数（std.io）による Rust システム API の呼び出し
- ユーザー定義の native 関数

### 設計目標

| 目標 | 説明 |
|------|------|
| ゼロランタイムオーバーヘッド | コンパイル時バインディングで、キャッシュ後は検索なし |
| 型安全性 | コンパイラが関数シグネチャをチェック |
| 拡張性 | ユーザーが任意の native 関数を宣言可能 |
| 新構文なし | 既存の `name: type = value` モデルを流用 |

### アーキテクチャ概要

```
┌─────────────────────────────────────────────────────────┐
│  コンパイル時                                             │
│  ───────────────────────────────────────────────────    │
│                                                          │
│  YaoXiang ソースコード:                                   │
│  read_file: (path: String) -> String = Native("...")   │
│                           │                              │
│                           ▼                              │
│  コンパイラが Native("name") 式を認識                    │
│                           │                              │
│                           ▼                              │
│  CallNative { func_id: "name" } バイトコードを生成      │
│                                                          │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│  ランタイム                                               │
│  ───────────────────────────────────────────────────    │
│                                                          │
│  CallNative { "std.io.read_file" }                     │
│       │                                                 │
│       ▼                                                 │
│  FfiRegistry.call() → キャッシュ検索 → 実行              │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## ステップ 1：FFI 登録表インフラストラクチャの作成

### ファイル

`src/backends/interpreter/ffi.rs`（新規作成）

### 実装内容

| 内容 | 説明 |
|------|------|
| `NativeHandler` 型 | `fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>` |
| `FfiRegistry` 構造体 | `handlers: HashMap<String, NativeHandler>` + キャッシュ |
| `with_std()` メソッド | std.io 関連関数を事前登録 |
| `register()` メソッド | ユーザーが新しい関数を登録 |
| `call()` メソッド | キャッシュ付き関数呼び出し |

### コアコード構造

```rust
pub struct FfiRegistry {
    // 関数ハンドラテーブル
    handlers: HashMap<String, NativeHandler>,
    // ランタイムキャッシュ（呼び出し高速化）
    cache: Mutex<HashMap<String, NativeHandler>>,
}

impl FfiRegistry {
    // 事前定義された標準ライブラリ関数
    pub fn with_std() -> Self { ... }

    // ユーザーが新しい関数を登録
    pub fn register(&mut self, name: &str, handler: NativeHandler) { ... }

    // 呼び出し：キャッシュ検索 → 実行
    pub fn call(&self, name: &str, args: &[RuntimeValue]) -> Result<RuntimeValue> { ... }
}
```

### 検収方法

- [x] `FfiRegistry::new()` が std.io 関数を含む登録表を返す
- [x] `register()` が新しい関数を追加できる
- [x] `call()` が登録済み関数を正しく呼び出せる

### テスト内容

- ユニットテスト：カスタム関数の登録と呼び出し ✅ 12/12 通過
- 統合テスト：事前登録された std.io 関数の呼び出し ✅

---

## ステップ 2：CallNative バイトコード命令の追加

### ファイル

`src/middle/core/bytecode.rs`

### 実装内容

| 内容 | 説明 |
|------|------|
| `Opcode::CallNative` | 新規オペコード |
| `CallNative` 命令構造体 | `dst: Option<Reg>, func_name: ConstIndex` |
| シリアライズ/デシリアライズ | バイトコードファイルの書き込みと読み込みをサポート |

### 検収方法

- [x] バイトコードが `CallNative` 命令を正しくシリアライズできる
- [x] デシリアライズ後、命令が正しい

### テスト内容

- シリアライズ：`CallNative { func_name: "test" }` をエンコード ✅
- デシリアライズ：デコード後、元の命令と一致 ✅

---

## ステップ 3：コード生成器が Native 関数を認識

### ファイル

`src/middle/passes/codegen/translator.rs`

### 実装内容

| 内容 | 説明 |
|------|------|
| `Native("name")` 式の認識 | `translate_call` 内で検出 |
| `CallNative` 命令の生成 | `CallStatic` を置換 |
| `Native` 型宣言の処理 | シンボルテーブルで `is_native: true` をマーク |

### 検収方法

- [x] `Native("std.io.read_file")` が `CallNative` バイトコードを生成
- [x] 通常関数は引き続き `CallStatic` を生成

### テスト内容

- コード生成テスト：`read_file("a.txt")` を `CallNative` に翻訳 ✅
- 関数呼び出しテスト：複数の引数が正しく渡される ✅

---

## ステップ 4：インタープリタが CallNative を実行

### ファイル

`src/backends/interpreter/executor.rs`

### 実装内容

| 内容 | 説明 |
|------|------|
| `Interpreter` への `FfiRegistry` の統合 | メンバー `ffi: FfiRegistry` として |
| `BytecodeInstr::CallNative` の処理 | `self.ffi.call()` を呼び出す |
| 引数変換 | `RuntimeValue` → Rust 型 → 返り値 |

### 検収方法

- [x] インタープリタが `CallNative` 命令を実行できる
- [x] 呼び出し結果が正しく返される

### テスト内容

- エンドツーエンドテスト：`println("hello")` が stdout に出力 ✅
- ファイルテスト：`write_file("test.txt", "content")` がファイルを作成 ✅
- エラー処理：存在しない native 関数がエラーを投げる ✅

---

## ステップ 5：Native 型サポートの型チェック

### ファイル

`src/frontend/typecheck/mod.rs`

### 実装内容

| 内容 | 説明 |
|------|------|
| `Native` 型注釈の認識 | 型推論時に処理 |
| 型シグネチャ検証 | 呼び出しシグネチャが登録と一致することを確認 |

### 検収方法

- [x] `Native("name")` が値として正しい型を持つ
- [x] 関数呼び出しの型チェックが通る

### テスト内容

- 型チェックテスト：正しいシグネチャが通る ✅
- 型エラー tests：引数数が不一致の場合エラー ✅

---

## ステップ 6：std.io インターフェースのリファクタリング

### ファイル

`src/std/io.rs`

### 実装内容

| 内容 | 説明 |
|------|------|
| 関数宣言の変更 | `Native("std.io.xxx")` 方式を使用 |
| ドキュメントコメント | 既存のドキュメントを維持 |

### 未実装関数

| 関数 | Native 名称 | 説明 |
|------|-------------|------|
| `print` | `std.io.print` | stdout に出力 |
| `println` | `std.io.println` | 出力して改行 |
| `read_file` | `std.io.read_file` | ファイル内容を読み込み |
| `write_file` | `std.io.write_file` | ファイルに書き込み |
| `read_line` | `std.io.read_line` | 1行を読み込み |
| `append_file` | `std.io.append_file` | 追加書き込み |

### 検収方法

- [x] `import std.io` 後、`read_file`、`write_file` などを呼び出せる ✅

### テスト内容

- 統合テスト：実際のファイル読み込み/書き込み ✅
- 機能テスト：各種 IO 関数が正常工作 ✅
- ユニットテスト：NativeDeclaration 登録表の検証 ✅ 6/6 通過
- ドキュメントテスト：NativeDeclaration の例が通る ✅

---

## ステップ 7：ユーザー定義 native 関数サポート

### ファイル

`src/std/ffi.rs`（新規作成）

### 実装内容

| 内容 | 説明 |
|------|------|
| `Native` 型定義 | ユーザーが native 関数を宣言するためのマーク |
| `register` 関数 | ユーザーが自分の native 関数処理ロジックを登録 |

### ユーザーの使用方法

**YaoXiang ソースコードでの宣言：**

```yaoxiang
# native 関数バインディングの宣言
my_add: (a: Int, b: Int) -> Int = Native("my_add")

# 呼び出し（コンパイラが自動的に CallNative バイトコードを生成）
result = my_add(1, 2)
```

**Rust 組み込み API での登録：**

```rust
// Rust 側で native 関数処理ロジックを登録
interpreter.ffi_registry_mut().register("my_add", |args| {
    let a = args[0].to_int().unwrap_or(0);
    let b = args[1].to_int().unwrap_or(0);
    Ok(RuntimeValue::Int(a + b))
});
```

### 実装内容

| 内容 | 説明 |
|------|------|
| `NativeBinding` 構造体 | ユーザーが宣言した native 関数バインディング (func_name → native_symbol) |
| `detect_native_binding()` | AST 内の `Native("...")` パターンを検出 |
| `ModuleIR.native_bindings` | IR 層で native バインディング情報を伝播 |
| IR ジェネレータ統合 | `= Native("symbol")` を検出後、関数体生成をスキップしてバインディングを記録 |
| Translator 統合 | `translate_module` 開始前にユーザーの native 関数を自動的に登録 |

### 検収方法

- [x] ユーザーがカスタム native 関数を宣言できる ✅
- [x] 登録後、正しく呼び出せる ✅

### テスト内容

- ユニットテスト：NativeBinding の作成とマッピング ✅ 6/6 通過
- 統合テスト：detect_native_binding パターン認識 ✅
- ドキュメントテスト：NativeBinding の例が通る ✅

---

## 依存関係

```
ステップ 1 (FFI 登録表)
    │
    ├── ステップ 4 (インタープリタ統合)
    │       │
    │       └── ステップ 6 (std.io リファクタリング)
    │
    ├── ステップ 2 (バイトコード)
    │       │
    │       └── ステップ 3 (コード生成)
    │               │
    │               └── ステップ 5 (型チェック)
    │
    └── ステップ 7 (ユーザー定義)
```

## 検収総覧

| ステップ | 検収条件 | ステータス |
|------|----------|------|
| 1 | FfiRegistry が作成・登録・呼び出し可能 | ✅ |
| 2 | バイトコードが正しくシリアライズ/デシリアライズ | ✅ |
| 3 | Native 式が CallNative を生成 | ✅ |
| 4 | インタープリタが実行し正しい結果を返す | ✅ |
| 5 | 型チェックが Native を正しく処理 | ✅ |
| 6 | std.io 関数が利用可能 | ✅ |
| 7 | ユーザー定義 native 関数サポート | ✅ |

## エンドツーエンドテスト結果

```
running 19 tests
- backends::interpreter::ffi::tests::test_new_registry_is_empty ... ok
- backends::interpreter::ffi::tests::test_with_std_has_io_functions ... ok
- backends::interpreter::ffi::tests::test_register_custom_function ... ok
- backends::interpreter::ffi::tests::test_call_custom_function ... ok
- backends::interpreter::ffi::tests::test_call_nonexistent_function_returns_error ... ok
- backends::interpreter::ffi::tests::test_call_println_via_registry ... ok
- backends::interpreter::ffi::tests::test_cache_accelerates_repeated_calls ... ok
- backends::interpreter::ffi::tests::test_register_overwrites_existing ... ok
- backends::interpreter::ffi::tests::test_registered_functions_list ... ok
- backends::interpreter::ffi::tests::test_write_and_read_file ... ok
- backends::interpreter::ffi::tests::test_read_file_missing_args ... ok
- backends::interpreter::ffi::tests::test_write_file_missing_args ... ok
- backends::interpreter::executor::tests::test_ffi_println_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_write_and_read_file_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_custom_function_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_nonexistent_function_e2e ... ok
- backends::interpreter::executor::tests::test_ffi_append_file_e2e ... ok

test result: ok. 19 passed; 0 failed; 0 ignored
```

### テストカバレッジ

- ✅ FFI 登録表の作成と登録
- ✅ 標準ライブラリ関数 (std.io.print, println, read_file, write_file, append_file)
- ✅ カスタム native 関数の登録と呼び出し
- ✅ エラー処理（存在しない関数）
- ✅ キャッシュ高速化
- ✅ ファイル読み書き
- ✅ ファイル追加