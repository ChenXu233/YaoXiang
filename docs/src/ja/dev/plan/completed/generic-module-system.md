# YaoXiang 汎用モジュールシステムリファクタリング計画

## 1. 概要

現在の YaoXiang のモジュールシステムは `std` 組み込みモジュール専用の特殊実装であり、複数の場所にハードコードされており、ユーザ定義モジュールのサポートがありません。本ドキュメントでは、汎用的なモジュールシステムを実装する方法について説明します。

## 2. 受け入れ基準

### 2.1 必須でサポートする機能

| 機能 | 構文例 | 説明 |
|------|---------|------|
| モジュールインポート | `use my_module` | ファイルシステムからモジュールを読み込む |
| 選択的インポート | `use my_module.{func1, func2}` | 指定した関数のみをインポート |
| モジュールのエイリアス | `use my_module as m` | エイリアスを使用してアクセス |
| モジュール呼び出し | `my_module.func()` | モジュール経由で関数を呼び出す |
| サブモジュール | `my_module.sub.func()` | サブモジュールにアクセス |
| モジュールの定数 | `my_module.CONST` | モジュールの定数にアクセス |

### 2.2 std モジュールの互換性

リファクタリング後も現在と同じ動作を維持する必要があります：

| 構文 | 例 | ステータス |
|------|------|------|
| `std.io.print()` | `use std.io` + `std.io.println("x")` | ✅ 維持必須 |
| `print()` (選択的インポート) | `use std.io.{print}` + `print("x")` | ✅ 維持必須 |
| `io.print()` (モジュールインポート) | `use std.{io}` + `io.println("x")` | ✅ 維持必須 |
| `std.math.PI` | 定数アクセス | ✅ 維持必須 |

### 2.3 モジュールの読み込みルール

```
// モジュールの検索パス（現在のファイルからの相対パス）
1. ./my_module.yx           // カレントディレクトリ
2. ./my_module/mod.yx      // サブディレクトリ
3. ./my_module/index.yx    // index ファイル

// std モジュールの特殊処理
// std モジュールは src/std/ にあり、常に利用可能
```

## 3. テスト要件

### 3.1 機能テスト

```yaoxiang
// test_user_module.yx
// my_module.yx ファイルが存在すると仮定

// 1. 基本的なモジュールインポート
use my_module
main = {
    my_module.greet()  // モジュール関数を呼び出す
}

// 2. 選択的インポート
use my_module.{add, sub}
main = {
    add(1, 2)
}

// 3. モジュールのエイリアス
use my_module as m
main = {
    m.greet()
}

// 4. サブモジュール
use my_module.utils
main = {
    my_module.utils.help()
}

// 5. ネストされたインポート
use my_module.{sub1, sub2.sub3}
```

### 3.2 std モジュールの互換性テスト

すべての既存のテスト用例が引き続き通過する必要があります：

```yaoxiang
// 既存の構文テスト
use std.io
main = { std.io.println("test") }

use std.io.{print}
main = { print("test") }

use std.{io}
main = { io.println("test") }

use std.math
main = { std.math.PI }
```

### 3.3 エッジケーステスト

- 空のモジュールインポート
- モジュールの循環依存（エラーを出す必要がある）
- 存在しないモジュール（エラーを出す必要がある）
- 重複インポート
- インポート衝突

## 4. 既存コードの分析

### 4.1 コードの匂いの位置

| ファイル | 行番号 | 問題 |
|------|------|------|
| `src/std/mod.rs` | 29-100 | ハードコードされた `get_module_exports`、std モジュールのみを処理 |
| `src/frontend/typecheck/mod.rs` | 593-622 | ハードコードされた std モジュールの処理ロジック |
| `src/frontend/typecheck/inference/expressions.rs` | 515-598 | ハードコードされた `io\|math\|net\|concurrent` リスト |
| `src/middle/core/ir_gen.rs` | 65-131 | ハードコードされた名前空間の処理 |

### 4.2 主要構造の分析

#### 4.2.1 ModuleExport (src/std/mod.rs)

```rust
pub struct ModuleExport {
    pub short_name: &'static str,      // 短い名前
    pub qualified_name: &'static str, // 完全なパス
    pub signature: &'static str,      // 関数シグネチャ
}
```

**問題**: std モジュール専用であり、汎用モジュールのエクスポートに一般化する必要がある

#### 4.2.2 NativeDeclaration (src/std/io.rs など)

```rust
pub struct NativeDeclaration {
    pub name: &'static str,
    pub native_name: &'static str,
    pub signature: &'static str,
    pub doc: &'static str,
    pub implemented: bool,
}
```

**説明**: `NativeDeclaration` は FFI（Foreign Function Interface）のために設計されており、YaoXiang から Rust 関数を呼び出すために使用されます。ユーザーは std.ffi `native` 関数を使用して Rust との相互運用を実現します。

**変更不要**: ユーザーモジュールは YaoXiang 自体で記述されるため、NativeDeclaration は不要です。モジュールシステムは YaoXiang ソースファイルを読み込むだけで済みます。

**設計の分離**：

| モジュールの種類 | 実装方式 | 読み込み方式 |
|---------|---------|----------|
| FFI モジュール | Rust + NativeDeclaration | 組み込み登録 |
| ユーザーモジュール | YaoXiang ソースファイル | ファイルシステムから読み込み |

#### 4.2.3 StmtKind::Use (AST)

```rust
StmtKind::Use {
    path: String,           // モジュールのパス
    items: Option<Vec<String>>, // インポート項目
    alias: Option<String>,     // エイリアス
}
```

**現状**: パーサーは完全な構文をサポートしているが、型チェックが完全に実装されていない

## 5. 実施計画

### 5.1 フェーズ 1: 汎用モジュールインターフェースを設計する

**目標**: 汎用的なモジュールの登録とクエリインターフェースを定義する

**変更が必要な可能性のあるファイル**:
- 新規作成 `src/frontend/module.rs` - モジュールシステムのコアインターフェース

**設計内容**:

```rust
// モジュールのエクスポート項目
pub struct ModuleExport {
    pub name: String,           // エクスポート名
    pub full_path: String,     // 完全なパス
    pub kind: ExportKind,      // 関数/定数/サブモジュール
    pub type_info: TypeInfo,   // 型情報
}

// モジュールのレジストリ
pub trait ModuleRegistry {
    fn get_module(&self, path: &str) -> Option<Module>;
    fn register_module(&mut self, path: String, module: Module);
}

// モジュールのローダー
pub trait ModuleLoader {
    fn load_module(&self, path: &str) -> Result<Module, ModuleError>;
}
```

### 5.2 フェーズ 2: モジュールローダーを実装する

**目標**: ファイルシステムからユーザーモジュールを読み込む

**変更が必要な可能性のあるファイル**:
- 新規作成 `src/frontend/module/loader.rs` - ファイルシステムモジュール読み込み
- 修正 `src/frontend/compiler.rs` - モジュール読み込みを統合

**実装内容**:
- モジュールの検索パスを実装
- モジュールの解決（AST 生成）を実装
- モジュールのキャッシュを実装

### 5.3 フェーズ 3: 型チェックのリファクタリング

**目標**: ハードコードを汎用モジュールシステムで置き換える

**変更が必要な可能性のあるファイル**:
- `src/frontend/typecheck/mod.rs` - 汎用モジュールインターフェースを使用
- `src/frontend/typecheck/inference/expressions.rs` - ハードコードを削除
- `src/std/mod.rs` - ModuleRegistry trait を実装

### 5.4 フェーズ 4: IR 生成のリファクタリング

**目標**: ハードコードされた名前空間の処理を削除

**変更が必要な可能性のあるファイル**:
- `src/middle/core/ir_gen.rs` - 汎用モジュールインターフェースを使用

### 5.5 フェーズ 5: std モジュールの適応

**目標**: std モジュールの互換性を確保

**変更が必要な可能性のあるファイル**:
- `src/std/mod.rs` - 新しいモジュールインターフェースに適応

### 5.6 フェーズ 6: テストと検証

**目標**: すべての機能が正常に動作することを確認

**新規作成が必要なファイル**:
- `tests/modules/` - モジュールシステムのテスト

## 6. アーキテクチャ設計（草案）

```
┌─────────────────────────────────────────────────────────────────┐
│                        コンパイラフロントエンド                    │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐   │
│  │   Parser     │───▶│ TypeChecker  │───▶│  IR Gen      │   │
│  └──────────────┘    └──────────────┘    └──────────────┘   │
│         │                   │                   │              │
│         ▼                   ▼                   ▼              │
│  ┌──────────────────────────────────────────────────────┐    │
│  │                   Module System                       │    │
│  ├──────────────────────────────────────────────────────┤    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐ │    │
│  │  │  Registry   │  │  Loader     │  │  Resolver   │ │    │
│  │  │  (レジストリ) │  │  (ローダー) │  │  (名前解決)  │ │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘ │    │
│  └──────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

## 7. 循環依存の処理

### 7.1 設計上の決定：積極的にエラーにする

モジュールシステムは**積極的にエラーにする**戦略を採用し、循環依存が検出されるとすぐにコンパイルを中止し、明確なエラー情報を表示します。

**理由**:
- 明確：ユーザーは即座に問題の所在を知る
- 有益：错误情報に循環パスを表示できる
- 直感的：Rust/Go などの言語の做法に準拠

### 7.2 検出アルゴリズム

```rust
// 1. 依存グラフを構築する
//    各モジュール -> そのモジュールが依存するモジュールのリスト

// 2. トポロジカルソート（カーン算法）
//    - 次数が0のノードを優先
//    - ソートできない = 循環が存在

// 3. ソートに失敗した場合、循環依存を報告する
```

### 7.3 循環の種類

#### 直接循環

```yaoxiang
// a.yx
use b

// b.yx
use a  // Error: 循環依存 a <-> b
```

#### 間接循環

```yaoxiang
// a.yx
use b

// b.yx
use c

// c.yx
use a  // Error: 間接循環 a -> b -> c -> a
```

#### 自己参照

```yaoxiang
// a.yx
use a  // Error: 自己参照
```

### 7.4 エラーメッセージの例

```yaoxiang
error[E1001]: cyclic dependency detected
  --> a.yx:1:1
   |
1  | use b
   | ^^^^^^
   |
note: dependency path: a -> b -> c -> a
help: break the cycle by removing one of these imports
```

### 7.5 実装の場所

- **モジュールローダー** (`src/frontend/module/loader.rs`)
- モジュールグラフの構築後すぐに検出
- コンパイル前期に検出，以降の作業を浪費しない

## 8. その他の制約

### 8.1 モジュールのバージョン

現時点ではモジュールのバージョン管理は考慮しません。

### 8.2 条件付きコンパイル

現時点では条件付きコンパイル（feature flags）は考慮しません。

## 9. モジュールのキャッシュ戦略

### 9.1 設計目標

RFC-014 のプロジェクト構造を参照し、開発時のホットリロードをサポートします：

```
my-project/
├── yaoxiang.toml
├── yaoxiang.lock
├── src/
│   └── main.yx
└── .yaoxiang/
    └── vendor/
        ├── foo-1.2.3/
        │   └── src/
        │       └── foo.yx
        └── bar-0.5.0/
```

### 9.2 キャッシュの種類

| キャッシュの種類 | タイミング | 戦略 |
|---------|------|------|
| **コンパイル時キャッシュ** | コンパイル中 | メモリキャッシュ，同一コンパイルユニット内で再利用 |
| **開発キャッシュ** | `yaoxiang run` 実行時 | ファイルシステムキャッシュ，ファイル変更時に自動リロード |
| **リリースキャッシュ** | リリースビルド | バージョン固定，ファイル変更を監視しない |

### 9.3 ホットリロード機構

```rust
// 開発モード：ファイル監視
struct HotReloader {
    watcher: notify::Watcher,  // ファイルシステム監視
}

// トリガー条件：
// 1. 依存する .yx ファイルが変更された
// 2. yaoxiang.toml または yaoxiang.lock が変更された

// リロード戦略：
// - 変更されたモジュールの增量再コンパイル
// - 依存グラフの再構築
// - 循環依存の検出
```

### 9.4 実装の場所

- **モジュールキャッシュ**: `src/frontend/module/cache.rs`
- **ホットリロード**: `src/frontend/module/hot_reload.rs`
- **統合ポイント**: `src/frontend/compiler.rs`

---

## 10. コンパイルエラー情報の最適化

### 10.1 設計目標

Rust のエラー情報の詳細さを参考に、明確で有益なエラー提示を提供します。

### 10.2 エラーの種類と例

#### 10.2.1 モジュールが見つからない

```yaoxiang
error[E1001]: module not found: 'my_module'
  --> main.yx:1:1
   |
1  | use my_module
   | ^^^^^^^^^^^^^^
   |
help: check if the file exists at one of these locations:
  - ./my_module.yx
  - ./my_module/index.yx
  - ./.yaoxiang/vendor/my_module-*/src/my_module.yx
note: you may need to add 'my_module' to your dependencies in yaoxiang.toml
```

#### 10.2.2 関数がエクスポートされていない

```yaoxiang
error[E1002]: export not found: 'undefined_func'
  --> main.yx:3:5
   |
3  | my_module.undefined_func()
   |     ^^^^^^^^^^^^^^^^^^^^
   |
help: 'my_module' exports these functions:
  - greet(name: String) -> String
  - add(a: i64, b: i64) -> i64
  - sub(a: i64, b: i64) -> i64
```

#### 10.2.3 循環依存

```yaoxiang
error[E1003]: cyclic dependency detected
  --> a.yx:1:1
   |
1  | use b
   | ^^^^^
   |
note: dependency path: a -> b -> c -> a
help: break the cycle by removing one of these imports
```

#### 10.2.4 パス解決エラー

```yaoxiang
error[E1004]: invalid module path
  --> main.yx:1:1
   |
1  | use ./relative/path
   | ^^^^^^^^^^^^^^^^^^
   |
help: use absolute module names (e.g., use my_module) or configure in yaoxiang.toml
```

### 10.3 実装の場所

- **エラー定義**: `src/util/diagnostic/error_codes.rs`
- **エラー生成**: 各モジュールシステムコンポーネント

---

## 11. モジュールパスの解決ルール

### 11.1 設計根拠

RFC-014 の 125-135 行目のモジュール解決順序の設計を参照。

### 11.2 パス構文

| 構文 | 例 | 説明 |
|------|------|------|
| 標準パス | `use foo.bar` | vendor/ と src/ を検索 |
| サブモジュール | `use foo.bar.baz` | ネストされたモジュール |
| カレントモジュール | `use .` | カレントモジュール（self） |
| ペアレントモジュール | `use ..` | ペアレントモジュール（parent） |

**サポート外**:
- 相対パス（`use ./utils`）
- 絶対パス（`use /usr/local/lib`）
- 文字列パッケージ名（`use "@org/package"`）

### 11.3 検索順序

```
use foo.bar.baz;

検索順序:
1. ./.yaoxiang/vendor/*/src/foo/bar/baz.yx  (vendor/ ディレクトリ)
2. ./src/foo/bar/baz.yx                      (ローカルモジュール)
3. $YXPATH/foo/bar/baz.yx                    (グローバルパス，予約済み)
4. $YXLIB/std/foo/bar/baz.yx                 (標準ライブラリ)
```

### 11.4 std モジュールの特殊処理

```
use std.io          ->  $YXLIB/std/io/ にマッピング
use std.math        ->  $YXLIB/std/math/ にマッピング
```

### 11.5 実装の場所

- **モジュール解決**: `src/frontend/module/resolver.rs`
- **パス検索**: `src/frontend/module/loader.rs`

---

## 12. 統一インターフェース設計

### 12.1 設計目標

std モジュールとユーザーモジュールは拡張可能な統一されたモジュールインターフェースを使用します。

### 12.2 コアインターフェース

```rust
// モジュール
pub trait Module {
    fn path(&self) -> &str;
    fn exports(&self) -> &HashMap<String, Export>;
}

// モジュールのレジストリ
pub trait ModuleRegistry {
    fn get(&self, path: &str) -> Option<Box<dyn Module>>;
    fn register(&mut self, path: String, module: Box<dyn Module>);
}

// モジュールのローダー
pub trait ModuleLoader {
    fn load(&self, path: &str) -> Result<Box<dyn Module>, ModuleError>;
}
```

### 12.3 具体的な実装

| 実装 | 用途 |
|------|------|
| `StdModule` | std 標準ライブラリ（組み込み） |
| `UserModule` | ユーザー定義モジュール（ファイル読み込み） |
| `VendorModule` | .yaoxiang/vendor/ 依存関係 |
| `CompositeRegistry` | 複数のレジストリを統合，検索順序でクエリ |

### 12.4 コンポジットレジストリ

```rust
struct CompositeRegistry {
    // 検索順序：前の方が優先
    std: StdModule,
    vendor: VendorRegistry,
    user: FileModuleRegistry,
}

impl ModuleRegistry for CompositeRegistry {
    fn get(&self, path: &str) -> Option<Box<dyn Module>> {
        // 順序で各レジストリを試行
        self.std.get(path)
            .or_else(|| self.vendor.get(path))
            .or_else(|| self.user.get(path))
    }
}
```

### 12.5 実装の場所

- **インターフェース定義**: `src/frontend/module/mod.rs`
- **std の実装**: `src/frontend/module/std.rs`
- **ファイル読み込み**: `src/frontend/module/file.rs`
- **コンポジター**: `src/frontend/module/registry.rs`

---

## 13. 実施チェックリスト

- [x] モジュールのキャッシュ戦略（`src/frontend/module/cache.rs`）
- [x] ホットリロード機構（`src/frontend/module/hot_reload.rs`）
- [x] コンパイルエラー情報（E5001-E5007 が定義済み）
- [x] モジュールパスの解決（`src/frontend/module/resolver.rs`）
- [x] 統一モジュールインターフェース（`src/frontend/module/mod.rs`）
- [x] std モジュールの適応（`src/frontend/module/registry.rs` + `src/std/mod.rs`）
- [x] 循環依存の検出（`src/frontend/module/loader.rs` カーン算法）
- [x] 型チェックのリファクタリング - ハードコードの削除（`src/frontend/typecheck/mod.rs`）
- [x] IR 生成のリファクタリング - ハードコードの削除（`src/middle/core/ir_gen.rs`）
- [x] 式推論のリファクタリング - ハードコードの削除（`src/frontend/typecheck/inference/expressions.rs`）
- [x] ユーザーモジュールのファイル読み込み（`src/frontend/module/loader.rs`，.yx ファイルを解析してエクスポート項目を抽出）
- [x] RFC-014 の統合（`src/frontend/module/vendor.rs`，vendor/、yaoxiang.toml）

---

## 14. 完了した実施記録

### 14.1 フェーズ 1：汎用モジュールインターフェース（完了 ✅）

**新規ファイル**:
- `src/frontend/module/mod.rs` - モジュールシステムのコア型定義（`Export`, `ModuleInfo`, `ModuleSource`, `ModuleError` など）
- `src/frontend/module/registry.rs` - 統一モジュールレジストリ（`ModuleRegistry`），std モジュールの自動検出
- `src/frontend/module/resolver.rs` - モジュールパス解決（`ModuleResolver`），std/vendor/ユーザーモジュールの検索をサポート
- `src/frontend/module/loader.rs` - モジュールローダー（`ModuleLoader`），循環依存の検出をサポート

**設計上の決定**:
- トレイトオブジェクトではなくデータ駆動型（`ModuleInfo` + `HashMap`）を使用，インターフェースを簡素化
- `ModuleRegistry::with_std()` は各 std モジュールの `native_declarations()` からエクスポート項目を自動検出
- `ModuleSource` 列挙型で `Std`/`User`/`Vendor` モジュールのソースを区別

### 14.2 フェーズ 2：ハードコードの削除（完了 ✅）

**変更したファイルと削除したハードコード**:

| ファイル | 削除したハードコード | 代替案 |
|------|------------|--------|
| `src/std/mod.rs` | `match module_path { "std" => ..., "std.io" => ... }` | `ModuleRegistry::with_std()` に委譲 |
| `src/frontend/typecheck/mod.rs` | `let std_modules = ["std.io", "std.math", ...]` | `registry.std_submodule_names()` で動的に取得 |
| `src/frontend/typecheck/mod.rs` | `crate::std::get_module_exports(path)` Use の処理内 | `self.env.module_registry.get(path)` でクエリ |
| `src/frontend/typecheck/inference/expressions.rs` | `matches!(name.as_str(), "io" \| "math" \| "net" \| "concurrent")` | `std_submodules` リストで動的に判断 |
| `src/middle/core/ir_gen.rs` | `use crate::std::{concurrent, io, math, net}` + 手動マッピング構築 | `ModuleRegistry::with_std()` で `NATIVE_NAMES`/`SHORT_TO_QUALIFIED`/`STD_SUBMODULES` を自動生成 |

### 14.3 フェーズ 3：错误コードの定義（完了 ✅）

**新規エラーコード**:
- `E5005` - 無効なモジュールパス
- `E5006` - 重複インポート
- `E5007` - モジュールエクスポートのヒント

### 14.4 フェーズ 4：ユーザーモジュールのファイル読み込み（完了 ✅）

**変更したファイル**:
- `src/frontend/module/loader.rs` - `load_from_file()` を実装，tokenize → parse → `extract_exports()` でエクスポート項目を抽出

**エクスポート抽出ルール**:

| 文の種類 | エクスポート条件 | ExportKind |
|---------|---------|-----------|
| `pub fn_name: ...` | `is_pub = true` | Function |
| `Name: Type = ...` | 常時エクスポート | Type |
| `name = expr`（不変） | `is_mut = false` | Constant |
| `mut name = expr` | エクスポートなし | — |

**新規ヘルパー関数**:
- `format_type()` - AST 型ノードをシグネチャ文字列にフォーマット

### 14.5 フェーズ 5：RFC-014 の統合（完了 ✅）

**新規ファイル**:
- `src/frontend/module/vendor.rs` - VendorBridge，`PackageManifest` + `VendorManager` とモジュールシステムを橋渡し

**ワークフロー**:
1. `yaoxiang.toml` を読み込んで宣言された依存関係を取得
2. `.yaoxiang/vendor/` ディレクトリをスキャンしてインストール済みの依存関係を見つける
3. 各依存関係のエントリファイルを解析してエクスポート項目を抽出
4. `ModuleRegistry` に登録

### 14.6 フェーズ 6：モジュールキャッシュ戦略（完了 ✅）

**新規ファイル**:
- `src/frontend/module/cache.rs` - スレッドセーフなモジュールキャッシュ（`parking_lot::RwLock`）

**キャッシュモード**:

| モード | 変更検出 | 適用シナリオ |
|------|---------|--------|
| `Compile` | 検出なし | コンパイル中のメモリキャッシュ |
| `Development` | FNV-1a ファイルハッシュ | 開発時の自動無効化 |
| `Release` | 検出なし | 本番ビルド |

### 14.7 フェーズ 7：ホットリロード機構（完了 ✅）

**新規ファイル**:
- `src/frontend/module/hot_reload.rs` - ファイル監視 + デバウンス + キャッシュ無効化

**新規依存関係**:
- `notify = "7.0.0"`（ファイルシステム監視）

**アーキテクチャ**:

```text
FileWatcher (notify) → デバウンス (300ms) → classify_events → invalidate_cache → ReloadEvent チャンネル
```

**イベントの分類**:

| ファイル変更 | FileChange | キャッシュ処理 |
|---------|-----------|--------|
| `.yx` 変更 | SourceModified | invalidate_by_file |
| `.yx` 作成 | SourceCreated | 処理不要 |
| `.yx` 削除 | SourceDeleted | invalidate_by_file |
| `yaoxiang.toml` | ManifestChanged | clear() |
| `yaoxiang.lock` | LockfileChanged | clear() |

### 14.8 検証結果

- ✅ 全部で 1494 テストが通過（1458 ユニット + 6 ドキュメント + 30 統合、0 失敗）
- ✅ `cargo check` コンパイルエラーなし
- ✅ すべてのチェックリスト項目が完了