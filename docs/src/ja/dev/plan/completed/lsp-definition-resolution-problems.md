# LSP 定義解決の問題とソリューション

> **タスク**：LSP ジャンプ定義機能の修正
> **日付**：2026-02-28
> **ステータス**：✅ 完了
> **発見日時**：2026-02-28
> **完了日時**：2026-03-01

---

## 概要

このドキュメントは、LSP サーバーで「定義へジャンプ」（Go to Definition）機能に関連する問題とソリューションを記録するものです。

---

## 問題 1：Use ステートメントでインポートされたシンボルが見つからない

### 問題の説明

`use std.list` ステートメントでインポートされたモジュールシンボル（`push`、`pop` など）は、ジャンプ定義機能では見つけることができません。

### 根本原因分析

**位置**：`src/lsp/world.rs:98-169`

```rust
// update_index_from_ast はこれらのステートメントタイプのみを処理
StmtKind::Var { ... }      // ✅ 処理済み
StmtKind::Fn { ... }       // ✅ 処理済み
StmtKind::TypeDef { ... }  // ✅ 処理済み
StmtKind::MethodBind { ... } // ✅ 処理済み
StmtKind::Use { ... }      // ❌ 処理なし！
```

**フローの問題**：
1. ユーザーコードが `use std.list` でモジュールをインポート
2. 型チェック段階でインポートされたシンボルが `TypeEnvironment` に登録される
3. しかし `update_index_from_ast` は AST トップレベルステートメントのみを解析し、**Use ステートメントを処理しない**
4. そのため Use でインポートされたシンボルは `SymbolIndex` に入らない
5. LSP ジャンプ定義時にこれらのシンボルが見つからない

### 影響

- ユーザーが `use std.list` を使用しても `std.list.push` などの関数定義にジャンプできない
- 補完機能も標準庫関数の定義を提供できない

---

## 問題 2：同名の関数が誤った位置にジャンプする

### 問題の説明

同じ関数名が複数のファイルに存在する場合、定義へのジャンプはすべての同名定義を返し、間違った位置に正確にジャンプすることができません。

### 根本原因分析

**位置**：`src/lsp/handlers/definition.rs:51-66`

```rust
// 現在の実装：名前だけでマッチングし、すべての同名定義を返す
let symbols = world.symbol_index().find_by_name(&ident.name);

let locations: Vec<Location> = symbols
    .iter()
    .filter_map(|sym| { ... })  // 名前が同じであれば返すだけで、コンテキストは考慮しない
    .collect();
```

**問題**：
1. `SymbolIndex` にはトップレベルシンボル（Var, Fn, TypeDef, MethodBind）のみが含まれる
2. 検索時に以下のことを考慮しない：
   - シンボルの型（変数 vs 関数）
   - シンボルのスコープ（ローカル vs グローバル）
   - 呼び出し時の型コンテキスト
3. すべての同名定義を返し、クライアントが選択（通常是第一个）

### 影響

- 複数のファイルに同名関数が存在する場合、誤った場所にジャンプする可能性がある
- ユーザーが手動で正しい定義を選択する必要がある

---

## 問題 3：ローカル変数と関数引数が見つからない

### 問題の説明

関数内部で定義されたローカル変数と関数引数は、ジャンプ定義機能では見つけることができません。

### 根本原因分析

**位置**：`src/lsp/world.rs:107-168`

```rust
// update_index_from_ast はモジュールトップレベルステートメントのみを処理
for stmt in &module.items {
    match &stmt.kind {
        StmtKind::Var { name, .. } => {
            // トップレベル変数のみを処理
        }
        StmtKind::Fn { name, params, .. } => {
            // 関数定義のみを処理し、関数本体の引数やローカル変数は処理しない
        }
        // ...
    }
}
```

**問題**：
1. `SymbolIndex` は**モジュールトップレベルステートメント**からのみシンボルを抽出
2. 関数引数、ローカル変数などの**ネストされたスコープ内のシンボル**がインデックスに入らない
3. LSP ジャンプ定義時にこれらのシンボルが見つからない

---

## 問題 4：標準庫に YaoXiang ソースファイルがない

### 問題の説明

標準庫（std.list、std.io など）は Rust で実装されており、対応する .yx ソースファイルがありません。

### 根本原因分析

**現在のアーキテクチャ**：

```
ユーザコード (.yx)              標準庫 (Rust)
      │                         │
      ▼                         ▼
  解析 → AST             StdModule → NativeExport
      │                         │
      ▼                         ▼
 SymbolIndex ◄──────── ModuleRegistry
                           (LSP からは見えない)
```

**問題**：
1. 標準庫は `StdModule` trait を介して `ModuleRegistry` に登録される
2. 各モジュールには `NativeExport` リストがあり、以下を含む：
   - `name`: 短い名前（例："push"）
   - `native_name`: 完全修飾名（例："std.list.push"）
   - `signature`: 関数シグネチャ（例："(list: List, item: Any) -> List"）
   - **注意**：`Span` がない（Rust 関数には YaoXiang ソース位置がない）
3. LSP サーバーは `ModuleRegistry` の存在を知らない
4. そのため標準庫関数の定義位置を特定できない

---

## ソリューション

### 方案 A：標準庫 YaoXiang インターフェースファイル

**コアアイデア**：標準庫向けに YaoXiang インターフェースファイルを作成し、ExternalBindingStmt を使用して Rust 関数にバインドする。

**ファイル構造**：

```
~/.yaoxiang/std/                    # インストールディレクトリ（グローバル標準庫）
├── list.yx                          # list モジュールインターフェース
│   push: (list: List, item: Any) -> List = ...
│   pop: (list: List) -> Any = ...
├── io.yx                            # io モジュールインターフェース
│   print: (...args) -> () = ...
│   println: (...args) -> () = ...
│   read_line: () -> String = ...
│   read_file: (path: String) -> String = ...
│   write_file: (path: String, content: String) -> Bool = ...
│   append_file: (path: String, content: String) -> Bool = ...
│   format_fallback: (value: Any, type_name: String) -> String = ...
├── dict.yx
├── string.yx
└── ...

プロジェクトディレクトリ/
├── main.yx
└── .yaoxiang/
    └── vendor/
        └── std/                     # プロジェクトローカルの標準庫（グローバルを上書き）
            └── list.yx              # オプション：グローバルの list インターフェースを上書き
```

**インターフェースファイルの形式**：

```yaoxiang
// io.yx - 標準庫 IO モジュールインターフェース
// LSP ジャンプと型表示のみに使用、実際の実行には関与しない


print: (...args) -> () = {
    // 標準出力に出力
    ... // 実装は Rust 側が提供
}


println: (...args) -> () = {
    // 標準出力に出力して改行
    ...
}


read_line: () -> String = {
    // 標準入力から1行を読み取る
    ...
}

read_file: (path: String) -> String = {
    /* ファイル内容を読み取る
    @param path ファイルパス */
    ...
}

write_file: (path: String, content: String) -> Bool = {
    /* ファイル内容を書き込む、既存のものを上書き
    @param path ファイルパス
    @param content ファイル内容
    @return 成功したかどうか */
    ...
}


append_file: (path: String, content: String) -> Bool = {
    /* ファイル内容を引き続き書き込む
    @param path ファイルパス
    @param content ファイル内容
    @return 成功したかどうか */
    ...
}

format_fallback: (value: Any, type_name: String) -> String = {
    /* 任意の型を文字列にフォーマット
    @param value 任意の値
    @param type_name 値の型名
    @return フォーマット後の文字列 */
    ...
}
```

**構文説明**：

- 等号の右側に `...` を使用して実際の実装をスキップすることを示す
- 関数ドキュメントを追加する必要がある場合は、ブロック構文を使用：
  ```yaoxiang
  print: (...args) -> () = {
      // ドキュメントコメント
      ...
  }
  ```

**モジュール検索順序**（Python と同様）：

```
use std.list の検索順序：
1. プロジェクトディレクトリ/.yaoxiang/vendor/std/list.yx  ← 優先（存在すれば使用）
2. ~/.yaoxiang/std/list.yx               ← フォールバック（デフォルト）
```

**メリット**：

- ユーザーコードと標準庫が同じ構文を使用
- LSP はこれらのファイルを直接解析してジャンプと補完を提供できる
- 標準庫インターフェース自体がドキュメント
- メンテナンスが容易
- プロジェクトローカルのグローバル標準庫上書きをサポート

**実装手順**：

1. **コード生成ツールの написа**：Rust コードの `NativeExport` から `.yx` インターフェースファイルを自動的に生成するコード生成ツールを作成
   - 入力：`src/std/io.rs` の `NativeExport` 定義
   - 出力：`.yaoxiang/std/io.yx` インターフェースファイル
   - 生成ルール：
     - `name` → 関数名
     - `signature` → 型注釈
     - `native_name` → 書き込まない（Rust バインディングのみに使用）

2. **ビルドプロセスへの統合**：Cargo ビルド時にコード生成ツールを自動的に実行
   ```rust
   // build.rs または独立の生成スクリプト
   fn main() {
       // src/std/*.rs を読み取る
       // NativeExport 定義を解析
       // .yaoxiang/std/*.yx ファイルを生成
   }
   ```

3. **モジュール解決ロジックの修正**：ダブルパスの検索をサポート（プロジェクト優先 → グローバルフォールバック）

4. **LSP サーバーの修正**：インターフェースファイルをシンボルインデックスにロードし、ジャンプと補完を提供

---

### 方案 B：同名関数の精密マッチングの修正

**コアアイデア**：SemanticDB の型情報とスコープ情報を使用して精密マッチングを行う。

**現在利用可能なリソース**：

- `SemanticDB`：より正確なシンボル情報を含む（型、スコープ）
- `symbol_defs`：シンボル名 → 定義位置リスト
- `symbol_refs`：シンボル名 → 参照位置リスト

**実装手順**：
1. `handle_definition` 関数を修正し、`SemanticDB` を使用した検索を優先
2. カーソル位置のコンテキスト（式型、スコープ）を活用して精密マッチング
3. `SemanticDB` で見つからない場合は `SymbolIndex` にフォールバック

---

### 方案 C：ローカル変数と関数引数の処理

**コアアイデア**：関数内部のシンボルもインデックスに追加する。

**実装手順**：
1. `update_index_from_ast` を修正し、関数本体を走査
2. 関数引数とローカル変数を抽出
3. 各シンボルにそのスコープレベルを記録
4. 検索時にスコープ情報を使用してフィルタリング

---

## 推奨優先度

| 優先度 | 問題 | ソリューション |
|--------|------|----------|
| P1 | 同名関数のジャンプエラー | 方案 B：SemanticDB 精密マッチングを使用 |
| P2 | ローカル変数が見つからない | 方案 C：シンボルインデックスの範囲を拡張 |
| P3 | Use インポートシンボルが見つからない | 方案 A：標準庫インターフェースファイルを作成 |
| P4 | 標準庫が見つからない | 方案 A：標準庫インターフェースファイルを作成 |

---

## 関連コード位置

| ファイル | 説明 |
|------|------|
| `src/lsp/handlers/definition.rs` | 定義へのジャンプ処理関数 |
| `src/lsp/world.rs` | シンボルインデックス更新ロジック |
| `src/frontend/core/lexer/symbols.rs` | SymbolIndex 定義 |
| `src/frontend/typecheck/semantic_db.rs` | SemanticDB 定義 |
| `src/frontend/typecheck/mod.rs` | 型チェッカー（Use ステートメントを処理） |
| `src/std/mod.rs` | 標準庫モジュール定義 |
| `src/frontend/module/registry.rs` | モジュールの登録 |

---

## 実装プロセス

### 問題の依存関係

```
┌─────────────────────────────────────────────────────────────┐
│                        実装依存関係図                       │
└─────────────────────────────────────────────────────────────┘

問題 4：標準庫インターフェースファイル
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 1. コード生成ツール（NativeExport → .yx ファイル） │
    │  │ 2. モジュールダブルパス検索（プロジェクト → グローバル）│
    │  │ 3. LSP がインターフェースファイルをシンボルインデックスにロード│
    │  └─────────────────────────────────────────────────────┘
    ▼
問題 1：Use シンボル位置特定
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 4. update_index_from_ast が Use ステートメントを処理│
    │  │ 5. use でインポートされたシンボルをインデックスに追加│
    │  └─────────────────────────────────────────────────────┘
    ▼
問題 2：同名関数のジャンプエラー  ←─┐
    │                         │
    │  ┌──────────────────┐   │
    │  │ 6. SemanticDB    │   │
    │  │    精密マッチング│   │
    │  └──────────────────┘   │
    │                         │
問題 3：ローカル変数が見つからない  ──┘
    │
    │  ┌─────────────────────────────────────────────────────┐
    │  │ 7. シンボルインデックスの範囲を拡張（関数本体を走査）│
    │  │ 8. スコープレベルの記録                             │
    │  └─────────────────────────────────────────────────────┘
    ▼
   すべて完了
```

---

## 実装優先度

| 順序 | 問題 | 複雑度 | 理由 | ステータス |
|------|------|--------|------|------|
| 1 | 問題 4：標準庫インターフェース | 中 | インフラ、他の問題が依存する可能性 | ✅ 完了 |
| 2 | 問題 1：Use シンボル位置特定 | 低 | 修正により標準庫ジャンプが使用可能に | ✅ 完了 |
| 3 | 問題 2：同名関数のジャンプ | 中 | 検索アルゴリズムの改善 | ✅ 完了 |
| 4 | 問題 3：ローカル変数 | 高 | 関数本体の走査が必要 | ✅ 完了 |

---

## 実装記録

### 完了日：2026-03-01

### 問題 2 の実装：SemanticDB 精密マッチング

**修正ファイル**：`src/lsp/handlers/definition.rs`

**実装内容**：

- `handle_definition` 関数をリファクタリングし、2段階検索戦略を採用：
  1. **SemanticDB を優先使用**：`try_semantic_db_lookup` を通じて `symbol_defs` を使用して精密な定義位置を検索
  2. **SymbolIndex へのフォールバック**：SemanticDB に結果がない場合、グローバルのシンボルインデックスを使用
- 新規 `try_semantic_db_lookup` 関数：
  - `SemanticDB.find_innermost_scope()` を使用してカーソル位置の最内側スコープを検索
  - スコープ内の `symbols` 情報を使用してコンテキストフィルタリング
  - 同一ファイル内の定義を優先マッチングし、誤ったジャンプを減少
- 複数の同名定義がある場合、現在のファイル優先度でソート

**新規テスト**：

- `test_definition_via_semantic_db`：SemanticDB 精密マッチングを検証
- `test_definition_semantic_db_disambiguates`：多定義解消scheme を検証

---

### 問題 3 の実装：ローカル変数と関数引数のインデックス

**修正ファイル**：`src/lsp/world.rs`

**実装内容**：

- `update_index_from_ast` メソッドを拡張し、関数本体を再帰的に走査：
  - 関数引数（`Param`）をシンボルインデックスに抽出
  - 関数体/メソッド体内のステートメントと式を走査
- 新規再帰ヘルパーメソッド：
  - `index_stmt_symbols`：ネストされたステートメントを処理（Var, For, If, ネストされた Fn など）
  - `index_expr_symbols`：式内のシンボル定義を処理（Lambda, FnDef, For, ListComp など）
  - `index_block_symbols`：ブロックレベルコードを走査

**新規テスト**：

- `test_update_index_fn_params`：関数引数が正しくインデックスに追加されることを検証

---

### 問題 1 の実装：Use ステートメントシンボル位置特定

**修正ファイル**：`src/lsp/world.rs`

**実装内容**：

- `update_index_from_ast` に `StmtKind::Use` ブランチ処理を追加
- 新規 `index_use_symbols` メソッド：
  - `ModuleRegistry::with_std()` を使用してモジュールエクスポートを検索
  - `use std.io`（すべてのエクスポートをインポート）をサポート
  - `use std.io.{println}`（指定アイテムをインポート）をサポート
  - インポートされたシンボルを現在のファイルのシンボルインデックスに追加

**新規テスト**：

- `test_update_index_use_stmt`：`use std.io` がすべてのエクスポートをインポートすることを検証
- `test_update_index_use_stmt_with_items`：指定アイテムのインポートを検証

---

### 問題 4 の実装：標準庫インターフェースファイル生成

**新規ファイル**：`src/std/gen_interfaces.rs`

**実装内容**：

- `generate_interface_content`：`StdModule::exports()` から `.yx` インターフェースファイルの内容を自動生成
- `generate_all_interfaces`：すべての10個の標準庫モジュールの一括インターフェース生成
- `write_interfaces_to_dir`：指定ディレクトリにインターフェースファイルを書き込む
- `default_std_interface_dir`：グローバル標準庫インターフェースディレクトリを取得（`~/.yaoxiang/std/`）
- `find_std_interface_file`：ダブルパス検索（プロジェクトローカル → グローバルフォールバック）

**LSP 統合**（`src/lsp/world.rs`）：

- 新規 `load_std_library_symbols` メソッド：
  - `all_module_infos()` から標準庫エクスポートを取得
  - `.yx` インターフェースファイルが存在する場合、解析して精密な Span 情報を取得
  - それ以外の場合は仮想パス（`std://std.io` など）と `Span::dummy()` を使用
- 新規 `parse_interface_file_spans` 関数：シンプルなテキスト解析で関数名行番号マッピングを取得
- LSP サーバー起動時（`src/lsp/server.rs`）に `load_std_library_symbols` を自動的に呼び出す

**インターフェースファイルの形式**：

```yaoxiang
// io.yx - 標準庫 std.io モジュールインターフェース
// LSP ジャンプと型表示のみに使用、実際の実行には関与しない

print: (...args) -> () = {
    ...
}

println: (...args) -> () = {
    ...
}
```

**モジュール検索順序**：

```
1. プロジェクトディレクトリ/.yaoxiang/vendor/std/<name>.yx  ← プロジェクトローカル（優先）
2. ~/.yaoxiang/std/<name>.yx                 ← グローバルフォールバック
```

**新規テスト**：

- `test_generate_all_interfaces`：10個モジュールのインターフェース生成を検証
- `test_io_interface_content`：io インターフェースの内容を検証
- `test_math_interface_has_constants`：定数エクスポートを検証
- `test_list_interface_content`：list インターフェースの内容を検証
- `test_write_interfaces_to_temp_dir`：ファイル書き込みを検証
- `test_find_std_interface_file`：パス検索を検証
- `test_load_std_library_symbols`：標準庫シンボルのロードを検証
- `test_parse_interface_file_spans`：インターフェースファイル Span 解析を検証

---

## 参照

- RFC-004：カリー化メソッドの複数位置ジョイントバインディング設計（ExternalBindingStmt）
- [Language Server Protocol 仕様](https://microsoft.github.io/language-server-protocol/)