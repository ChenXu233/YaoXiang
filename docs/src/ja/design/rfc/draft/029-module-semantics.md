---
title: 'RFC-029: モジュール意味システム'
status: '草案'
author: '晨煦'
created: '2026-06-13'
updated: '2026-07-14（改編：互換性セクションの削除、サブRFCへの問題の分離）'
---

# RFC-029: モジュール意味システム

## 摘要

モジュールシステムをコンパイルパイプラインに接続し、複数ファイルコンパイルとパッケージレベルの可視性制御を実現する。

**コア原則**：型チェッカーは事前構築されたモジュールレジストリのクエリのみを行い、ディスクには触れない。モジュールグラフは型チェックの前に完全に構築される。

**含まないもの**：キャッシュ、ファイル監視、HOTリロード、インクリメンタル再コンパイル。これらはコンパイルライフサイクルの最適化であり、後続の独立したRFCが担当する。

## 動機

### 現在の問題

1. **コンパイラは単一ファイルのみサポート**：`Compiler::compile(name, source)` はファイル間依存関係を処理できない
2. **エクスポートルールが競合**：型は自動エクスポート、定数は自動エクスポート、メソッドは自動エクスポート、関数は `pub` をチェック——4つの例外
3. **2つのモジュールリゾルバ**：`frontend/module/resolver.rs` と `package/source/module_resolver.rs`
   検索順序が異なる
4. **型チェッカーがファイルロードと密結合**：草案では `use` が型チェック時に `ModuleLoader::load()` をトリガーすることを要求していた

### 設計目標

- 1つのプロジェクトで複数の `.yx` ファイルをコンパイル可能
- `use` 文のセマンティクスが明確で曖昧さがない
- 可視性ルールを1つに統一
- 単一ファイルモードは引き続き動作し、`yaoxiang.toml` は不要
- 型チェッカーは純粋なロジックであり、ファイル I/O を実行しない

## 提案

### 1. モジュールのアイデンティティとパス解決

#### モジュールの定義

**モジュール**は `.yx` ファイルである。モジュールパスはドット区切りの名前パスであり、ファイルシステム上の位置に対応する。

```
math.geometry → src/math/geometry.yx
             → src/math/geometry/mod.yx
             → src/math/geometry/index.yx
```

**パッケージ**は `yaoxiang.toml` を持つプロジェクトであり、複数のモジュールを含む。パッケージは唯一の캡슐化境界である。

#### パス解決ルール

検索順序（唯一のルールとして、既存の2つのリゾルバを置き換え）：

1. **標準ライブラリ**：`std` または `std.*` → 組み込みモジュール、`ModuleRegistry` からクエリ
2. **vendor ディレクトリ**：`.yaoxiang/vendor/<pkg>-*/src/` → 依存パッケージ
3. **現在のファイルの相対パス**：現在の `.yx` ファイルの所在ディレクトリ相对于
4. **プロジェクト src ディレクトリ**：`<project_root>/src/`

ファイル配置的試行順序：

```
base/name.yx
base/name/mod.yx
base/name/index.yx
```

最初に見つかったファイルで停止する。`name.yx` と `name/mod.yx` が同時に存在する場合、エラーを報告：

```
モジュールパスが曖昧：`math.geometry` が同時にマッチ：
  src/math/geometry.yx
  src/math/geometry/mod.yx
どちらかを削除してください。
```

#### 統一リゾルバ

既存の2つの `ModuleResolver` を廃止する。`frontend/module/resolver.rs` を唯一の実装として残し、`package/source/module_resolver.rs` を削除する。`YXPATH` 環境変数のサポートは唯一のリゾルバに統合する。

### 2. インポート意味

#### 構文形式

```yaoxiang
use math.geometry                          # モジュール名前空間
use math.geometry as geo                   # モジュール名前空間エイリアス
use math.geometry.{Point}                  # 選択的インポート
use math.geometry.{Point, distance}        # 複数選択的インポート
use math.geometry.{Point as P}             # 選択的インポート＋エイリアス
use math.geometry.{Point as P, distance as dist}  # 複数選択的インポート＋エイリアス
```

#### 意味

すべてのインポート形式は**コンパイル時名前解決ルール**であり、実行時の参照コピーではない。インポートされた名前はモジュールエクスポートテーブル内の宣言アイデンティティを指す。

| 構文                       | 現在のスコープにバインド           | 使用方法         |
| -------------------------- | ---------------------------------- | ---------------- |
| `use path`                 | path の最後のセグメントをモジュール名前空間として | `geometry.Point` |
| `use path as alias`        | alias をモジュール名前空間として   | `alias.Point`    |
| `use path.{item}`          | item そのもの                     | `item`           |
| `use path.{item as alias}` | alias そのもの                     | `alias`          |

#### 削除された構文

- ~~`from path use item`~~：Python from-import 形式は採用しない
- ~~`use path.*`~~：ワイルドカードインポートは衝突リスクをもたらす、モジュール名前空間インポートで十分
- ~~`use path.{a, b} as c, d`~~：平行リストによる位置ベースのペアリングは脆弱なデータ構造であり、エイリアスは各宣言の後に付ける必要がある：`use path.{a as c, b as d}`

#### パスの意味

`use path` の `path` は常に**モジュールパス**であり、宣言ではない。モジュールが見つからない場合は即座にエラー：

```
モジュール `math.geometry.Point` が見つかりません。
`Point` がモジュール `math.geometry` 内の宣言である場合は、以下を使用：
use math.geometry.{Point}
```

「まず完全なモジュールを探し、失敗したら最後のセグメントを宣言として扱う」というフォールバックはしない。

#### インポート衝突

同名インポートは静かに上書きせず即座にエラー：

```
名前 `Point` のインポート衝突：
  math.geometry.Point
  graphics.geometry.Point
選択的インポートまたはモジュール名前空間エイリアスを使用してください。
```

### 3. 可視性

#### ルール

パッケージは唯一の캡슐화境界である。モジュールは権限境界不承担。

| 記述             | 現在のパッケージ内 | 他パッケージ |
| ---------------- | :----------------: | :---------: |
| デフォルト（`pub` なし） |        ✅        |      ❌     |
| `pub`            |        ✅        |      ✅     |

**1つのルール、すべてのトップレベル宣言に適用**：型、関数、定数、メソッド。

既存のコードの4つの例外を廃止：

- ~~型定義は常にエクスポート~~ → 同一ルール
- ~~定数は自動エクスポート~~ → 同一ルール
- ~~メソッドは自動エクスポート~~ → 同一ルール
- ~~関数は `pub` のみチェック~~ → 同一ルール

#### データ構造

AST の `is_pub: bool` を以下で置換：

```rust
pub enum Visibility {
    Package,  // デフォルト：現在のパackage 内のみで可視
    Public,   // pub：すべてのパッケージで可視
}
```

#### エクスポートテーブル

各モジュールは2つのテーブルを維持：

- **PackageSymbols**：すべてのパトップレベル宣言を含むパッケージ内完全シンボルテーブル
- **PublicExports**：他パッケージに提供する `pub` 宣言のサブセット

同一パッケージの `use` は `PackageSymbols` をクエリ；パッケージ間 `use` は `PublicExports` のみをクエリ可能。

パッケージ間での非 `pub` 宣言の参照は即座にエラー：

```
モジュール `math.geometry` の `internalHelper` は可視でない。
pub 宣言ではないため、`math` パッケージ内でのみ使用可能。
```

### 4. プロジェクトコンパイルフロー

#### コンパイルパイプライン

```
プロジェクトエントリポイント
  → yaoxiang.toml を読み込んでエントリファイルを取得
  → エントリから再帰的に use 文を解析し、すべての依存モジュールを発見
  → モジュールの依存グラフ（ModuleDependencyGraph）を構築
  → 循環依存を検出
  → トポロジカルソート
  → 各モジュールを順番に実行：字句解析 → 構文解析 → エクスポート抽出
  → ModuleRegistry を構築（すべてのモジュールのエクスポートテーブルを含む）
  → トポロジカル順序で各モジュールに対して型チェックを実行（ModuleRegistry をクエリ）
  → 複数の ModuleIR を生成
  → 診断を集約
```

型チェッカーは事前構築された `ModuleRegistry` を**のみクエリ**し、ファイルロードを実行せず、ディスクに触れない。

#### エントリファイルの選択

優先順位：

1. `[run].main`（存在する場合）
2. `[[bin]]` の最初の項目の `path`
3. `[lib].path`
4. `src/main.yx`（慣例によるデフォルト）

単一ファイルモードでは `yaoxiang.toml` は不要であり、指定されたファイルを直接コンパイルする。

#### 循環依存

```
循環依存を検出：
  math.geometry → math.transform → math.geometry
```

循環依存はコンパイルエラーであり、特殊処理はしない。

#### 誤った集約

複数ファイルコンパイルのエラーはモジュールのトポロジカル順序で集約される。各エラーにはソースモジュールとファイル位置を标注：

```
エラー：モジュール `math.geometry` 内：
  src/math/geometry.yx:12:5
  型 `Circle` が未定義

エラー：モジュール `app.main` 内：
  src/main.yx:3:1
  モジュール `math.geometry` が不可視
```

### 5. コンパイラの改动

| コンポーネント                          | 改动                                                                    |
| --------------------------------------- | ----------------------------------------------------------------------- |
| `compiler.rs`                           | 新規 `compile_project(project_root)` メソッドを追加                                            |
| `pipeline.rs`                           | 単一モジュールコンパイルの責務を維持し、神オブジェクトにはならない                           |
| `typecheck/checker.rs`                  | `use` 文は `ModuleRegistry` をクエリし、ファイルロードをトリガーしない                         |
| `typecheck/inference/statements.rs`     | 同上、`process_use_stmt` はクエリのみを行い、ロードはしない                                   |
| `frontend/module/resolver.rs`           | `package/source/module_resolver.rs` の YXPATH サポートを統合し、唯一のリゾルバとする             |
| `frontend/module/loader.rs`             | 拡張：再帰発見、完全なモジュールグラフ構築をサポート                                           |
| `frontend/module/dep_graph.rs`          | 実装済み、トポロジカルソートと循環検出を再利用                                               |
| `frontend/module/registry.rs`           | 実装済み、エクスポートテーブルクエリを再利用                                                   |
| `frontend/module/cache.rs`              | 実装済み、本 RFC ではコンパイルパイプラインに接続しない                                       |
| `frontend/module/hot_reload.rs`         | 実装済み、本 RFC ではコンパイルパイプラインに接続しない                                       |
| AST `is_pub: bool`                      | `Visibility` 列挙型で置換                                                                    |
| `package/source/module_resolver.rs`     | 削除、責務は `frontend/module/resolver.rs` に統合                                              |

## 実装戦略

### フェーズ分け

**Phase 1：モジュールの統一解決**

1. 2つの `ModuleResolver` を統合し、`package/source/module_resolver.rs` を削除
2. `YXPATH` 環境変数をサポート
3. モジュールパスの曖昧さ検出

**Phase 2：可視性データ構造**

4. AST `is_pub: bool` → `Visibility` 列挙型
5. リゾルバが `pub` キーワードを `Visibility::Public` にマッピングをサポート
6. `ModuleLoader::extract_exports` が統一的に `Visibility` を使用してエクスポートを判断

**Phase 3：プロジェクトコンパイルエントリポイント**

7. `compiler.rs` に新規 `compile_project(project_root)` メソッド
8. エントリから再帰的にモジュールを発見し、`ModuleDependencyGraph` を構築
9. トポロジカルソート、順序通りにモジュールをロードしてエクスポートを抽出
10. 完全な `ModuleRegistry` を構築
11. トポロジカル順序で各モジュールの型チェックを実行
12. 複数の `ModuleIR` を生成し、診断を集約

**Phase 4：インポート構文**

13. `use path.{item as alias}` 構文を実装
14. パス末尾のフォールバック推測を廃止

### 依存関係

- RFC-014（パッケージマネージャ）— 名前は `yaoxiang.toml` から、vendor ディレクトリ構造
- RFC-011（泛型システム）— trait は構造化型であり、モジュールの帰属には関係しない
- RFC-009（所有权モデル）— モジュールのインポートはコンパイル時名前解決であり、実行時の参照コピーには関係しない

## サブ RFC 計画

以下のサブ RFC は**予定計画**であり、まだ起草を開始していない：

| サブ RFC | 能力（予定）                    | 前提条件（予定）         |
| ------ | ------------------------------- | ------------------------ |
| 029a   | モジュールキャッシュとインクリメンタル再コンパイル            | モジュールのグラフとエクスポートテーブルが安定       |
| 029b   | ファイル監視とHOTリロード                | 029a のキャッシュ無効化メカニズム      |
| 029c   | 再エクスポート（`pub use`）             | エクスポートテーブルと可視性ルールが実装済み   |
| 029d   | CLI パラメータ `--entry` によるエントリ選択の上書き         | プロジェクトコンパイルエントリポイントが使用可能         |
| 029e   | 複数ファイル診断 `--json` 出力形式    | 診断集約メカニズムが使用可能         |
| —      | `pub(package)` モジュールの非公開可視性   | 現在の現実的需求がなく、一時的に含めない |
| —      | ワークスペース複数パッケージコンパイル                | RFC-014c が担当         |

## 参考文献

- [RFC-009: 所有権モデル](accepted/009-ownership-model.md) — Move セマンティクス、インポートはコンパイル時名前解決
- [RFC-011: 泛型型システム](accepted/011-generic-type-system.md) — 構造化型定義
- [RFC-014: パッケージ管理システム設計](accepted/014-package-manager.md) — パッケージ名のソース、vendor ディレクトリ
- [RFC-015: 設定システム](accepted/015-configuration-system.md) — `yaoxiang.toml` のフィールド定義
