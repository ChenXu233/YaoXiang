```markdown
---
title: "RFC-029: モジュール意味論システム"
status: "草案"
author: "晨煦"
created: "2026-06-13"
updated: "2026-07-14（書き直し：互換性章を削除、未解決問題はサブ RFC へ分割）"
---

# RFC-029: モジュール意味論システム

## 要約

モジュールシステムをコンパイルパイプラインに接続し、複数ファイルのコンパイルとパッケージレベルの可視性制御を実現する。

**基本原則**：型検査器は事前構築されたモジュール登録表（Module Registry）に問い合わせるのみで、ディスクには触れない。モジュールグラフは型検査の前に完全に構築される。

**含まないもの**：キャッシュ、ファイル監視、ホットリロード、インクリメンタル再コンパイル。これらはコンパイルライフサイクルの最適化であり、後続の独立した RFC で扱う。

## 動機

### 現状の問題

1. **コンパイラは単一ファイルのみサポート**：`Compiler::compile(name, source)` はファイル間の依存関係を処理できない
2. **エクスポートルールが衝突**：型の自動エクスポート、定数の自動エクスポート、メソッドの自動エクスポート、関数の `pub` チェック——四つの例外
3. **二つのモジュールリゾルバが存在**：`frontend/module/resolver.rs` と `package/source/module_resolver.rs` で検索順序が異なる
4. **型検査器がファイル読み込みと結合**：草案では `use` が型検査時に `ModuleLoader::load()` をトリガーすることを要求していた

### 設計目標

- 一つのプロジェクトで複数の `.yx` ファイルをコンパイル可能
- `use` 文のセマンティクスが明確で曖昧さがない
- 可視性ルールを一つに統一
- 単一ファイルモードは引き続き動作し、`yaoxiang.toml` を要求しない
- 型検査器は純粋な論理であり、ファイル I/O を実行しない

## 提案

### 1. モジュール ID とパス解決

#### モジュールの定義

**モジュール**は `.yx` ファイルである。モジュールパスはドット区切りの名前パスで、ファイルシステムの位置に対応する。

```
math.geometry → src/math/geometry.yx
             → src/math/geometry/mod.yx
             → src/math/geometry/index.yx
```

**パッケージ**は `yaoxiang.toml` を持つプロジェクトであり、複数のモジュールを含む。パッケージは唯一のカプセル化境界である。

#### パス解決ルール

検索順序（唯一のルール、既存の二つのリゾルバを置き換え）：

1. **標準ライブラリ**：`std` または `std.*` → 組み込みモジュール、`ModuleRegistry` に問い合わせ
2. **vendor ディレクトリ**：`.yaoxiang/vendor/<pkg>-*/src/` → 依存パッケージ
3. **現在のファイルの相対パス**：現在の `.yx` ファイルが置かれているディレクトリからの相対
4. **プロジェクトの src ディレクトリ**：`<project_root>/src/`

ファイル位置の試行順序：

```
base/name.yx
base/name/mod.yx
base/name/index.yx
```

最初に見つかったファイルで停止する。`name.yx` と `name/mod.yx` が同時に存在する場合、エラー：

```
モジュールパスが曖昧：`math.geometry` が同時にマッチ：
  src/math/geometry.yx
  src/math/geometry/mod.yx
いずれかを削除してください。
```

#### 統一リゾルバ

既存の二つの `ModuleResolver` を廃止する。`frontend/module/resolver.rs` を唯一の実装として残し、`package/source/module_resolver.rs` を削除する。`YXPATH` 環境変数のサポートは唯一のリゾルバに統合する。

### 2. インポートセマンティクス

#### 構文形式

```yaoxiang
use math.geometry                          # モジュール名前空間
use math.geometry as geo                   # モジュール名前空間エイリアス
use math.geometry.{Point}                  # 選択的インポート
use math.geometry.{Point, distance}        # 複数選択的インポート
use math.geometry.{Point as P}             # 選択的インポート with エイリアス
use math.geometry.{Point as P, distance as dist}  # 複数 with エイリアス
```

#### セマンティクス

すべてのインポート形式は**コンパイル時の名前解決ルール**であり、ランタイム参照のコピーではない。インポートされた名前は、モジュールエクスポート表の宣言 ID を指す。

| 構文 | 現在のスコープにバインド | 使用方法 |
|------|-----------------|----------|
| `use path` | path の最終セグメントをモジュール名前空間として | `geometry.Point` |
| `use path as alias` | alias をモジュール名前空間として | `alias.Point` |
| `use path.{item}` | item 自体 | `item` |
| `use path.{item as alias}` | alias 自体 | `alias` |

#### 削除される構文

- ~~`from path use item`~~：Python の from-import 形式は採用しない
- ~~`use path.*`~~：ワイルドカードインポートは競合リスクがあり、モジュール名前空間インポートで十分
- ~~`use path.{a, b} as c, d`~~：並列リストの位置ペアリングは脆弱なデータ構造。エイリアスは各宣言の後に続ける必要がある：`use path.{a as c, b as d}`

#### パスのセマンティクス

`use path` の `path` は常に**モジュールパス**であり、宣言ではない。モジュールが見つからない場合は直接エラー：

```
モジュール `math.geometry.Point` が見つかりません。
`Point` がモジュール `math.geometry` 内の宣言である場合は、以下を使用してください：
use math.geometry.{Point}
```

「最初に完全なモジュールを探し、失敗したら最後のセグメントを宣言とみなす」フォールバックは行わない。

#### インポート競合

同名のインポートは直接エラー、静かに上書きしない：

```
名前 `Point` のインポートが競合：
  math.geometry.Point
  graphics.geometry.Point
選択的インポートまたはモジュール名前空間エイリアスを使用してください。
```

### 3. 可視性

#### ルール

パッケージは唯一のカプセル化境界である。モジュールは権限境界を負わない。

| 書き方 | 現在のパッケージ内 | 他のパッケージ |
|------|:--------:|:------:|
| デフォルト（`pub` なし） | ✅ | ❌ |
| `pub` | ✅ | ✅ |

**全トップレベル宣言に適用される単一のルール**：型、関数、定数、メソッド。

既存のコードの四つの例外を廃止：

- ~~型定義は常にエクスポート~~ → 同じルール
- ~~定数は自動エクスポート~~ → 同じルール
- ~~メソッドは自動エクスポート~~ → 同じルール
- ~~関数のみ `pub` をチェック~~ → 同じルール

#### データ構造

AST の `is_pub: bool` を以下に置き換える：

```rust
pub enum Visibility {
    Package,  // デフォルト：現在のパッケージでのみ可視
    Public,   // pub：すべてのパッケージで可視
}
```

#### エクスポート表

各モジュールは二つの表を維持する：

- **PackageSymbols**：パッケージ内の完全なシンボル表、すべてのトップレベル宣言を含む
- **PublicExports**：他のパッケージに提供する `pub` 宣言のサブセット

同パッケージ内の `use` は `PackageSymbols` に問い合わせる；パッケージ間の `use` は `PublicExports` のみ問い合わせ可能。

パッケージ間で非 `pub` 宣言を参照すると直接エラー：

```
モジュール `math.geometry` の `internalHelper` は可視ではありません。
これは pub 宣言ではないため、`math` パッケージ内でのみ使用可能です。
```

### 4. プロジェクトコンパイルフロー

#### コンパイルパイプライン

```
プロジェクトエントリ
  → yaoxiang.toml を読み込み、エントリファイルを取得
  → エントリから再帰的に use 文を解析し、すべての依存モジュールを発見
  → モジュール依存グラフ（ModuleDependencyGraph）を構築
  → 循環依存を検出
  → トポロジカルソート
  → 各モジュールを順に実行：字句解析 → 構文解析 → エクスポート抽出
  → ModuleRegistry を構築（すべてのモジュールのエクスポート表を含む）
  → トポロジカル順に各モジュールの型検査を実行（ModuleRegistry に問い合わせ）
  → 複数の ModuleIR を生成
  → 診断を集約
```

型検査器は事前構築された `ModuleRegistry` に**問い合わせるのみ**で、ファイル読み込みを実行せず、ディスクに触れない。

#### エントリファイルの選択

優先順位：

1. `[run].main`（存在する場合）
2. `[[bin]]` の最初の項目の `path`
3. `[lib].path`
4. `src/main.yx`（デフォルト規約）

単一ファイルモードは `yaoxiang.toml` を必要とせず、与えられたファイルを直接コンパイルする。

#### 循環依存

```
循環依存が検出されました：
  math.geometry → math.transform → math.geometry
```

循環依存はコンパイルエラーであり、特別な処理は行わない。

#### エラー集約

複数ファイルコンパイルのエラーはモジュールトポロジカル順に集約される。各エラーはソースモジュールとファイル位置を注釈として持つ：

```
エラー：モジュール `math.geometry` において：
  src/math/geometry.yx:12:5
  型 `Circle` が未定義

エラー：モジュール `app.main` において：
  src/main.yx:3:1
  モジュール `math.geometry` は可視ではありません
```

### 5. コンパイラの改変

| コンポーネント | 改変 |
|------|------|
| `compiler.rs` | 新規メソッド `compile_project(project_root)` を追加 |
| `pipeline.rs` | 単一モジュールコンパイルの責務を維持、God Object にしない |
| `typecheck/checker.rs` | `use` 文は `ModuleRegistry` に問い合わせ、ファイル読み込みをトリガーしない |
| `typecheck/inference/statements.rs` | 同上、`process_use_stmt` は問い合わせのみで読み込みしない |
| `frontend/module/resolver.rs` | `package/source/module_resolver.rs` の YXPATH サポートを統合し、唯一のリゾルバになる |
| `frontend/module/loader.rs` | 拡張：再帰的発見、完全なモジュールグラフ構築をサポート |
| `frontend/module/dep_graph.rs` | 実装済み、トポロジカルソートと循環検出を再利用 |
| `frontend/module/registry.rs` | 実装済み、エクスポート表の問い合わせを再利用 |
| `frontend/module/cache.rs` | 実装済み、本 RFC ではコンパイルパイプラインに接続しない |
| `frontend/module/hot_reload.rs` | 実装済み、本 RFC ではコンパイルパイプラインに接続しない |
| AST `is_pub: bool` | `Visibility` 列挙型に置き換え |
| `package/source/module_resolver.rs` | 削除、責務は `frontend/module/resolver.rs` に統合 |

## 実装戦略

### フェーズ分け

**Phase 1：統一モジュール解決**
1. 二つの `ModuleResolver` を統合し、`package/source/module_resolver.rs` を削除
2. `YXPATH` 環境変数をサポート
3. モジュールパスの曖昧さ検出

**Phase 2：可視性データ構造**
4. AST `is_pub: bool` → `Visibility` 列挙型
5. パーサが `pub` キーワードを `Visibility::Public` にマッピングすることをサポート
6. `ModuleLoader::extract_exports` を統一して `Visibility` でエクスポートを判定

**Phase 3：プロジェクトコンパイルエントリ**
7. `compiler.rs` に新規メソッド `compile_project(project_root)` を追加
8. エントリから再帰的にモジュールを発見し、`ModuleDependencyGraph` を構築
9. トポロジカルソート、順にモジュールを読み込みエクスポートを抽出
10. 完全な `ModuleRegistry` を構築
11. トポロジカル順に各モジュールの型検査
12. 複数の `ModuleIR` を生成、診断を集約

**Phase 4：インポート構文**
13. `use path.{item as alias}` 構文を実装
14. パス末尾のフォールバック推測を廃止

### 依存関係

- RFC-014（パッケージマネージャ）— 名前は `yaoxiang.toml` から、vendor ディレクトリ構造
- RFC-011（generics）— trait は構造化型であり、モジュール帰属を伴わない
- RFC-009（所有権モデル）— モジュールインポートはコンパイル時名前解決であり、ランタイム参照のコピーを伴わない

## サブ RFC 計画

以下のサブ RFC は**計画中**であり、まだ起草を開始していない：

| サブ RFC | 能力（予定） | 前提条件（予定） |
|--------|-------------|-----------------|
| 029a | モジュールキャッシュとインクリメンタル再コンパイル | モジュールグラフとエクスポート表の安定 |
| 029b | ファイル監視とホットリロード | 029a のキャッシュ無効化メカニズム |
| 029c | 再エクスポート（`pub use`） | エクスポート表と可視性ルールの実装 |
| 029d | CLI 引数 `--entry` によるエントリ選択のオーバーライド | プロジェクトコンパイルエントリの利用可能性 |
| 029e | 複数ファイル診断 `--json` 出力形式 | 診断集約メカニズムの利用可能性 |
| — | `pub(package)` モジュールプライベート可視性 | 現在の現実的なニーズなし、暫定的に含めない |
| — | ワークスペースマルチパッケージコンパイル | RFC-014c が担う |

## 参考文献

- [RFC-009: 所有権モデル](accepted/009-ownership-model.md) — Move セマンティクス、インポートはコンパイル時名前解決
- [RFC-011: generics システム](accepted/011-generic-type-system.md) — 構造化型定義
- [RFC-014: パッケージ管理システム設計](accepted/014-package-manager.md) — パッケージ名の出所、vendor ディレクトリ
- [RFC-015: 設定システム](accepted/015-configuration-system.md) — `yaoxiang.toml` フィールド定義
```